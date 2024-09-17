use crate::ui::Rect;

#[derive(Default)]
pub struct Warpdrive {
    rect: Rect,
}

impl Warpdrive {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }
}
