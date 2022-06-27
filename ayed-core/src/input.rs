#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Return,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}
