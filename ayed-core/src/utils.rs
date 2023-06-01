#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn top_left(&self) -> (u32, u32) {
        (self.x, self.y)
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
