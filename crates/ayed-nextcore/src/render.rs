use crate::{rect::Rect, style::Style};

pub struct FullRender {
    pub panes: Vec<RenderPane>,
}

pub struct RenderPane {
    rect: Rect,
    lines: Vec<String>,
    styled_regions: Vec<StyledRegion>,
}

#[derive(Debug, Default, Clone)]
pub struct StyledRegion {
    pub rect: Rect,
    pub style: Style,
    pub priority: u8,
}

pub fn rectangle() {
    todo!("rectangle drawing helper")
}
