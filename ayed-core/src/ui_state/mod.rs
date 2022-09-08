use std::collections::{BTreeMap, BinaryHeap};

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
        enum SpanEdge {
            Start(usize),
            End(usize),
        }
        #[derive(Default)]
        struct NonOverlappingSpanEdge {
            end: Option<usize>,
            start: Option<usize>,
        }
        impl NonOverlappingSpanEdge {
            pub fn set_start(&mut self, start: usize) -> &mut Self {
                self.start = Some(start);
                self
            }
            pub fn set_end(&mut self, end: usize) {
                self.end = Some(end);
            }
        }

        fn heap_remove_by_id(
            id: usize,
            heap: BinaryHeap<ImportantSpan>,
        ) -> BinaryHeap<ImportantSpan> {
            heap.into_iter()
                .filter(|important_span| important_span.id != id)
                .collect()
        }

        let mut spans = std::mem::take(&mut self.spans);
        spans.sort_by(|a, b| a.from.cmp(&b.from));

        let mut span_edges_by_position: BTreeMap<Position, Vec<SpanEdge>> = BTreeMap::new();

        for (i, span) in spans.iter().enumerate() {
            let id = i;
            span_edges_by_position
                .entry(span.from)
                .or_default()
                .push(SpanEdge::Start(id));
            span_edges_by_position
                .entry(span.to)
                .or_default()
                .push(SpanEdge::End(id));
        }

        let mut spans_by_importance: BinaryHeap<ImportantSpan> = BinaryHeap::new();

        let mut non_overlapping_span_edges_by_position: BTreeMap<Position, NonOverlappingSpanEdge> =
            BTreeMap::new();

        for (&position, span_edges) in span_edges_by_position.range(..) {
            // Start all starting spans
            for span_edge in span_edges {
                match span_edge {
                    &SpanEdge::Start(current_span_edge_id) => {
                        let span = &spans[current_span_edge_id];
                        let importance = span.importance;

                        let maybe_previous_most_important = spans_by_importance.peek().cloned();
                        spans_by_importance
                            .push(ImportantSpan::new(current_span_edge_id, importance));
                        let current_most_important = spans_by_importance.peek().cloned().unwrap();

                        match maybe_previous_most_important {
                            None => {
                                // No previous ongoing span, starting edge can simply be inserted.
                                non_overlapping_span_edges_by_position
                                    .entry(position)
                                    .or_default()
                                    .set_start(current_span_edge_id);
                            }
                            Some(previous_most_important)
                                if previous_most_important.id == current_most_important.id =>
                            {
                                // The current starting span is not more important than a previous ongoing span, so nothing to do.
                            }
                            Some(previous_most_important) => {
                                if current_most_important.id == current_span_edge_id {
                                    // Spans are overlapping! and the current starting span is more important than an previous ongoing span.
                                    // Must insert previous ongoing span ending edge.
                                    // Must insert current span starting edge.
                                    non_overlapping_span_edges_by_position
                                        .entry(position)
                                        .or_default()
                                        .set_start(current_span_edge_id)
                                        .set_end(previous_most_important.id);
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }

            // End all ending spans
            let position = position.with_moved_indices(0, 1);
            for span_edge in span_edges {
                match span_edge {
                    &SpanEdge::End(current_span_edge_id) => {
                        let previous_ongoing_span = spans_by_importance.peek().cloned().unwrap();
                        spans_by_importance =
                            heap_remove_by_id(current_span_edge_id, spans_by_importance);
                        let maybe_current_ongoing_span = spans_by_importance.peek().cloned();

                        match maybe_current_ongoing_span {
                            None => {
                                // No current ongoing span, ending edge can simply be inserted.
                                non_overlapping_span_edges_by_position
                                    .entry(position)
                                    .or_default()
                                    .set_end(current_span_edge_id);
                            }
                            Some(current_ongoing_span)
                                if current_ongoing_span.id == previous_ongoing_span.id =>
                            {
                                // The current ending span is not more important than the now current ongoing span, so nothing to do.
                            }
                            Some(current_ongoing_span) => {
                                if previous_ongoing_span.id == current_span_edge_id {
                                    // Spans are overlapping! and the current ending span was more important than the now current ongoing span.
                                    // Must insert current span ending edge.
                                    // Must insert now current ongoing span starting edge.
                                    non_overlapping_span_edges_by_position
                                        .entry(position)
                                        .or_default()
                                        .set_start(current_ongoing_span.id)
                                        .set_end(current_span_edge_id);
                                }
                            }
                        }
                    }
                    _ => (),
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
        let mut starting_span = Span::default();
        let mut ending_span = Span::default();
        for (position, edge) in non_overlapping_span_edges_by_position {
            if let Some(_) = edge.end {
                ending_span.to = position.with_moved_indices(0, -1);
                normalized_spans.push(ending_span);
            }
            if let Some(start_edge_id) = edge.start {
                let original_span = &spans[start_edge_id];
                starting_span.style = original_span.style;
                starting_span.importance = original_span.importance;
                starting_span.from = position;
            }

            ending_span = starting_span;
            starting_span = Span::default();
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

#[derive(Debug, Clone, Copy)]
struct ImportantSpan {
    id: usize,
    importance: u8,
}

impl ImportantSpan {
    pub fn new(id: usize, importance: u8) -> Self {
        Self { id, importance }
    }
}

impl std::cmp::PartialOrd for ImportantSpan {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.importance.partial_cmp(&other.importance)
    }
}
impl std::cmp::Ord for ImportantSpan {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.importance.cmp(&other.importance)
    }
}

impl std::cmp::PartialEq for ImportantSpan {
    fn eq(&self, other: &Self) -> bool {
        self.importance.eq(&other.importance)
    }
}
impl std::cmp::Eq for ImportantSpan {}

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

        ui_panel.normalize_spans();

        //assert!(false);
    }
}
