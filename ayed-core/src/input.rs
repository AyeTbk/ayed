#[derive(Debug, Clone, Copy)]
pub enum Input {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}
