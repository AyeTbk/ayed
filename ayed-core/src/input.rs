#[derive(Debug, Clone, Copy)]
pub enum Input {
    Char(char),
    Return,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}
