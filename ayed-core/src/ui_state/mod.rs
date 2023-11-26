use std::collections::BTreeMap;

use crate::utils::{Position, Size};

pub struct UiState {
    pub panels: Vec<UiPanel>,
}

pub struct UiPanel {
    pub position: Position,
    pub size: Size,
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

        // Fill up list of span edges
        for span in spans.iter() {
            non_overlapping_subspans_by_position
                .entry(span.from)
                .or_default();
            non_overlapping_subspans_by_position
                .entry(span.to.with_moved_indices(0, 1))
                .or_default();
        }

        // Associate nonoverlapping subspans with the most important span's index that spans it
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

        // (Optional) Merge contiguous nonoverlapping subspans that share the same span index
        non_overlapping_subspans_by_position = {
            let mut non_overlapping_merged_subspans_by_position: BTreeMap<
                Position,
                Option<SpanIndex>,
            > = BTreeMap::new();

            let mut subspans = non_overlapping_subspans_by_position.into_iter()/*.peekable()*/;
            let mut previous_span_idx = None;
            while let Some((pos, span_idx)) = subspans.next() {
                if span_idx == previous_span_idx {
                    continue;
                }
                non_overlapping_merged_subspans_by_position.insert(pos, span_idx);
                previous_span_idx = span_idx;
            }

            non_overlapping_merged_subspans_by_position
        };

        // Extract normalized spans
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
            let is_before = span.from.row < line_index;
            let is_after = span.to.row > line_index;
            let is_same_line = span.from.row == line_index || span.to.row == line_index;
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
            position: (1, 1).into(),
            size: (1, 1).into(),
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
