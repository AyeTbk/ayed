use crate::token::TokenKind;

#[derive(Debug)]
pub struct Error<'a> {
    pub kind: ErrorKind,
    pub slice: &'a str,
}

impl<'a> Error<'a> {
    pub fn new(kind: ErrorKind, slice: &'a str) -> Self {
        Self { kind, slice }
    }

    pub fn is_recoverable(&self) -> bool {
        !self.is_eof_error()
    }

    pub fn is_eof_error(&self) -> bool {
        match self.kind {
            ErrorKind::Unexpected(Expected::TokenKind(kind))
            | ErrorKind::UnexpectedToken { got: kind, .. }
                if kind == TokenKind::Eof =>
            {
                true
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    UnexpectedToken { expected: Expected, got: TokenKind },
    Unexpected(Expected),
}

#[derive(Debug)]
pub enum Expected {
    TokenKind(TokenKind),
    Tag(&'static str),
}

impl From<TokenKind> for Expected {
    fn from(value: TokenKind) -> Self {
        Expected::TokenKind(value)
    }
}
