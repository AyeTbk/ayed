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
    // TODO Also split up spans across lines correctly
    /// Modify span list so that none are overlapping.
    pub fn normalize_spans(&mut self) {
        self.split_spans_over_lines();
        self.split_overlapping_spans();
        // self.merge_contiguous_compatible_spans(); potential TODO?
        self.make_spans_nonoverlapping();
    }

    fn split_spans_over_lines(&mut self) {
        // NOTE also look at TextBufferEdit::selections_split_by_lines
        let spans = std::mem::take(&mut self.spans);
        self.spans = spans
            .into_iter()
            .flat_map(|span| {
                (span.from.row..=span.to.row).map(move |row| {
                    let from_column = if row == span.from.row {
                        span.from.column
                    } else {
                        0
                    };
                    let to_column = if row == span.to.row {
                        span.to.column
                    } else {
                        u32::MAX
                    };

                    Span {
                        from: (from_column, row).into(),
                        to: (to_column, row).into(),
                        ..span
                    }
                })
            })
            .collect();
    }

    fn split_overlapping_spans(&mut self) {
        // TODO this to replace Self::make_spans_nonoverlapping?
        // Self::make_spans_nonoverlapping doesnt work right right now.
    }

    fn make_spans_nonoverlapping(&mut self) {
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
                .entry(span.to.with_moved_indices(1, 0))
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

        // Extract nonoverlapping spans
        let mut nonoverlapping_spans = Vec::new();
        let subspan_start = non_overlapping_subspans_by_position.iter();
        let subspan_end = non_overlapping_subspans_by_position
            .iter()
            .skip(1)
            .map(|(pos, _)| pos.with_moved_indices(-1, 0));
        let subspans = subspan_start.zip(subspan_end);

        for ((&start, &maybe_span_idx), end) in subspans {
            let span_idx = if let Some(span_idx) = maybe_span_idx {
                span_idx
            } else {
                continue;
            };
            let span = &spans[span_idx];
            nonoverlapping_spans.push(Span {
                from: start,
                to: end,
                style: span.style,
                importance: span.importance,
            });
        }

        self.spans = nonoverlapping_spans;
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
    // Spans position are relative to the panel's top-left.
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
    pub underlined: bool,
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
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
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

    pub fn from_hex(hex: &str) -> Result<Color, ()> {
        // Valid hexcodes are made exclusively of ascii characters, so working on bytes is ok.
        let hex = hex.as_bytes();
        if hex.len() != 7 {
            return Err(());
        }
        if hex[0] != b'#' {
            return Err(());
        }
        fn hex_digit_value(digit: u8) -> Option<u8> {
            match digit {
                b'a'..=b'f' => Some(digit - b'a' + 10),
                b'A'..=b'F' => Some(digit - b'A' + 10),
                b'0'..=b'9' => Some(digit - b'0'),
                _ => None,
            }
        }
        fn hex_value(first_char: u8, second_char: u8) -> Option<u8> {
            Some((hex_digit_value(first_char)? << 4) | hex_digit_value(second_char)?)
        }

        let r = hex_value(hex[1], hex[2]).ok_or(())?;
        let g = hex_value(hex[3], hex[4]).ok_or(())?;
        let b = hex_value(hex[5], hex[6]).ok_or(())?;

        Ok(Color::rgb(r, g, b))
    }
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
                    to: Position::new(15, 0),
                    style: Default::default(),
                    importance: 0,
                },
                Span {
                    from: Position::new(4, 0),
                    to: Position::new(10, 0),
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
