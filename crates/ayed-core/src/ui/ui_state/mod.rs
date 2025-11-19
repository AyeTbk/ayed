use crate::{position::Position, ui::Rect};

use super::{Style, layout::Size};

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
    pub fn prepare_for_render(&mut self) {
        self.fixup_weird_chars();
        self.spans.sort_by_key(|sr| -(sr.priority as i16));
    }

    fn fixup_weird_chars(&mut self) {
        for line in &mut self.content {
            // Tabs render in a special way in terminals, which doesn't match what the editor wants to show.
            // Tabs should be rendered as an appropriate amount of space by renderers.
            if line.find('\t').is_some() {
                *line = line.replace('\t', "⬸");
            }
        }
    }
}

pub struct PreparedUiPanel {
    pub position: Position,
    pub size: Size,
    pub content: Vec<String>,
    pub styled_regions: Vec<StyledRegion>,
    styled_regions_per_line: Vec<Vec<usize>>,
}

impl PreparedUiPanel {
    pub fn style_for_pos(&self, pos: Position) -> Style {
        let mut style = Style::default();
        for sr in self.styled_regions_for_pos(pos) {
            if style.foreground_color.is_none() {
                style.foreground_color = sr.style.foreground_color
            }
            if style.background_color.is_none() {
                style.background_color = sr.style.background_color
            }
            if !style.invert {
                style.invert = sr.style.invert
            }
            if !style.bold {
                style.bold = sr.style.bold
            }
            if !style.underlined {
                style.underlined = sr.style.underlined
            }
        }
        style
    }

    fn styled_regions_for_pos(&self, pos: Position) -> impl Iterator<Item = &StyledRegion> {
        let row: usize = pos.row.try_into().unwrap();
        let regions_for_row: &[usize] = self
            .styled_regions_per_line
            .get(row)
            .map(|v| &v[..])
            .unwrap_or_default();
        regions_for_row
            .iter()
            .flat_map(|&idx| self.styled_regions.get(idx))
            .filter(move |sr| Rect::from_positions(sr.from, sr.to).contains_position(pos))
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
