#[derive(Debug, Clone, Copy)]
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
