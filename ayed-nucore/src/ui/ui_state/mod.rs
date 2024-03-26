use std::collections::BTreeMap;

use crate::position::Position;

use super::{layout::Size, Style};

pub struct UiState {
    pub panels: Vec<UiPanel>,
}

pub struct UiPanel {
    pub position: Position,
    pub size: Size,
    pub content: Vec<String>,
    pub spans: Vec<StyledRegion>,
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

                    StyledRegion {
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
                    let other_span_priority = spans[*idx].priority;
                    if span.priority > other_span_priority {
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
            nonoverlapping_spans.push(StyledRegion {
                from: start,
                to: end,
                style: span.style,
                priority: span.priority,
            });
        }

        self.spans = nonoverlapping_spans;
    }

    pub fn spans_on_line(&self, line_index: u32) -> impl Iterator<Item = &StyledRegion> {
        self.spans.iter().filter(move |span| {
            let is_before = span.from.row < line_index;
            let is_after = span.to.row > line_index;
            let is_same_line = span.from.row == line_index || span.to.row == line_index;
            is_same_line || (is_before && is_after)
        })
    }
}

#[derive(Debug, Default, Clone)]
// FIXME change 'from' and 'to' to a Rect
pub struct StyledRegion {
    // Relative to the panel's top-left.
    pub from: Position,
    pub to: Position,
    pub style: Style,
    pub priority: u8,
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
                StyledRegion {
                    from: Position::new(0, 0),
                    to: Position::new(15, 0),
                    style: Default::default(),
                    priority: 0,
                },
                StyledRegion {
                    from: Position::new(4, 0),
                    to: Position::new(10, 0),
                    style: Default::default(),
                    priority: 2,
                },
            ],
        };

        // TODO test for warpdrive (ex: overlapping at line start)

        ui_panel.normalize_spans();

        //assert!(false);
    }
}
