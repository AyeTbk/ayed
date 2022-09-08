use std::collections::BTreeMap;

use crate::selection::Position;

pub struct UiState {
    pub panels: Vec<UiPanel>,
}

pub struct UiPanel {
    pub position: (u32, u32),
    pub size: (u32, u32),
    pub content: Vec<String>,
    pub spans: Vec<Span>,
}

impl UiPanel {
    /// Modify span list so that none are overlapping.
    pub fn normalize_spans(&mut self) {
        type SpanIndex = usize;

        let spans = std::mem::take(&mut self.spans);

        let mut non_overlapping_subspans_by_position: BTreeMap<Position, Option<SpanIndex>> =
            BTreeMap::new();

        for span in spans.iter() {
            non_overlapping_subspans_by_position
                .entry(span.from)
                .or_default();
            non_overlapping_subspans_by_position
                .entry(span.to.with_moved_indices(0, 1))
                .or_default();
        }

        for (span_idx, span) in spans.iter().enumerate() {
            let range = span.from..=span.to;
            for (_, most_important_span_idx) in
                non_overlapping_subspans_by_position.range_mut(range)
            {
                if let Some(idx) = most_important_span_idx {
                    let other_span_importance = spans[*idx].importance;
                    if span.importance > other_span_importance {
                        *idx = span_idx;
                    }
                } else {
                    *most_important_span_idx = Some(span_idx);
                }
            }
        }

        // DEBUG help
        // let mut cur_line_index = 0;
        // let mut strn = String::from("\n");
        // for (pos, edge) in non_overlapping_span_edges_by_position.range(..) {
        //     if pos.line_index > cur_line_index {
        //         strn += "\n";
        //         cur_line_index = pos.line_index;
        //     }
        //     strn += &format!(
        //         "  {:?}]({},{})[{:?}  ",
        //         edge.end, pos.line_index, pos.column_index, edge.start
        //     );
        // }
        // strn += "\n";
        // panic!("{}", strn);

        let mut normalized_spans = Vec::new();

        let subspan_start = non_overlapping_subspans_by_position.iter();
        let subspan_end = non_overlapping_subspans_by_position
            .iter()
            .skip(1)
            .map(|(pos, _)| pos.with_moved_indices(0, -1));
        let subspans = subspan_start.zip(subspan_end);

        for ((&start, &maybe_span_idx), end) in subspans {
            let span_idx = if let Some(span_idx) = maybe_span_idx {
                span_idx
            } else {
                continue;
            };
            let span = &spans[span_idx];
            normalized_spans.push(Span {
                from: start,
                to: end,
                style: span.style,
                importance: span.importance,
            });
        }

        self.spans = normalized_spans;
    }

    pub fn spans_on_line(&self, line_index: u32) -> impl Iterator<Item = &Span> {
        self.spans.iter().filter(move |span| {
            let is_before = span.from.line_index < line_index;
            let is_after = span.to.line_index > line_index;
            let is_same_line =
                span.from.line_index == line_index || span.to.line_index == line_index;
            is_same_line || (is_before && is_after)
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct Span {
    pub from: Position,
    pub to: Position,
    pub style: Style,
    pub importance: u8,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Style {
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
    pub invert: bool,
}

impl Style {
    pub fn with_foreground_color(&self, color: Color) -> Self {
        let mut this = *self;
        this.foreground_color = Some(color);
        this
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Self { r, g, b }
    }

    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };
    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_spans() {
        let mut ui_panel = UiPanel {
            position: (1, 1),
            size: (1, 1),
            content: Vec::new(),
            spans: vec![
                Span {
                    from: Position::new(0, 0),
                    to: Position::new(0, 15),
                    style: Default::default(),
                    importance: 0,
                },
                Span {
                    from: Position::new(0, 4),
                    to: Position::new(0, 10),
                    style: Default::default(),
                    importance: 2,
                },
            ],
        };

        // TODO test for warpdrive (ex: overlapping at line start)

        ui_panel.normalize_spans();

        //assert!(false);
    }
}
