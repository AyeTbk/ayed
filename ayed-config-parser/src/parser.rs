use crate::{
    ast::{Ast, Block, BlockKind, MappingBlock, MappingEntry, SelectorBlock, Span},
    error::Expected,
    token::{next_entry_value, next_token, Token, TokenKind},
    Error, ErrorKind,
};

pub struct Parser<'a> {
    src: &'a str,
    errors: Vec<Error<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            errors: Vec::new(),
        }
    }

    pub fn parse_module(mut self) -> (Ast<'a>, Vec<Error<'a>>) {
        let mut ast = Ast::default();

        loop {
            match self.parse_block() {
                Ok(block) => ast.top_level_blocks.push(block),
                Err(err) if err.is_eof_error() => break,
                Err(err) => {
                    let can_recover = err.is_recoverable();
                    self.add_error(err);
                    if !can_recover {
                        break;
                    }
                }
            }
        }

        (ast, self.errors)
    }

    fn parse_block(&mut self) -> Result<Block<'a>, Error<'a>> {
        let annotations = self.parse_annotations()?;
        let is_override = annotations.iter().any(|s| s.slice == "@override");
        if annotations.iter().any(|s| s.slice == "@raw") {
            todo!()
        }

        let name = self.expect(TokenKind::CharSoup)?;

        let lookahead = self.peek_token();
        let kind = match lookahead.kind {
            TokenKind::CharSoup => BlockKind::SelectorBlock(self.parse_selector_block(name)?),
            TokenKind::Delimiter => BlockKind::MappingBlock(self.parse_mapping_block(name)?),
            token_kind => {
                return Err(Error::new(
                    ErrorKind::Unexpected(token_kind.into()),
                    lookahead.slice,
                ))
            }
        };
        Ok(Block { is_override, kind })
    }

    fn parse_annotations(&mut self) -> Result<Vec<Span<'a>>, Error<'a>> {
        let mut annotations = Vec::new();
        loop {
            let lookahead = self.peek_token();
            if lookahead.slice.starts_with('@') {
                annotations.push(Span::from(lookahead.slice));
                self.read_token();
            } else {
                break;
            }
        }
        Ok(annotations)
    }

    fn parse_selector_block(&mut self, name: Token<'a>) -> Result<SelectorBlock<'a>, Error<'a>> {
        let pattern = self.expect(TokenKind::CharSoup)?;
        let children = self.parse_delimited_list(Self::parse_block, "{", "}")?;
        Ok(SelectorBlock {
            state_name: name.slice.into(),
            pattern: pattern.slice.into(),
            children,
        })
    }

    fn parse_mapping_block(&mut self, name: Token<'a>) -> Result<MappingBlock<'a>, Error<'a>> {
        let entries = self.parse_delimited_list(Self::parse_mapping_entry, "{", "}")?;
        Ok(MappingBlock {
            name: name.slice.into(),
            entries,
        })
    }

    fn parse_mapping_entry(&mut self) -> Result<MappingEntry<'a>, Error<'a>> {
        let name = self.expect(TokenKind::CharSoup)?;
        let (i, value) = next_entry_value(&self.src);
        self.src = i;
        Ok(MappingEntry {
            name: name.slice.into(),
            value: value.slice.into(),
        })
    }

    fn parse_delimited_list<T>(
        &mut self,
        parse_fn: impl Fn(&mut Self) -> Result<T, Error<'a>>,
        open: &'static str,
        close: &'static str,
    ) -> Result<Vec<T>, Error<'a>> {
        self.expect_delimiter(open)?;

        let mut items = Vec::new();
        'goto_end: {
            'looop: loop {
                match parse_fn(self) {
                    Ok(item) => {
                        items.push(item);
                    }
                    Err(err) => {
                        let can_recover = err.is_recoverable();
                        self.add_error(err);
                        if !can_recover {
                            break 'goto_end;
                        }
                        self.recover_delimited(open, close, 1);
                    }
                }

                if self.peek_token().slice == close {
                    break 'looop;
                }
            }

            match self.expect_delimiter(close) {
                Err(_) => {
                    unreachable!(
                        "cant leave above loop without the token being the close delimiter"
                    );
                }
                _ => (),
            }
        }
        Ok(items)
    }

    fn recover_delimited(&mut self, open: &'static str, close: &'static str, mut balance: i32) {
        loop {
            let peeked = self.peek_token().slice;

            if peeked == open {
                balance += 1;
            } else if peeked == close {
                balance -= 1;
            }

            if balance == 0 {
                break;
            }

            let token = self.read_token();
            if token.kind == TokenKind::Eof {
                break;
            }
        }
    }

    fn add_error(&mut self, err: Error<'a>) {
        self.errors.push(err);
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token<'a>, Error<'a>> {
        let token = self.read_token();
        if token.kind == kind {
            Ok(token)
        } else {
            Err(Error::new(
                ErrorKind::UnexpectedToken {
                    expected: kind.into(),
                    got: token.kind,
                },
                token.slice,
            ))
        }
    }

    fn expect_delimiter(&mut self, tag: &'static str) -> Result<Token<'a>, Error<'a>> {
        let token = self.read_token();
        match token.kind {
            TokenKind::Delimiter if token.slice == tag => Ok(token),
            _ => Err(Error::new(
                ErrorKind::UnexpectedToken {
                    expected: Expected::Tag(tag),
                    got: token.kind,
                },
                token.slice,
            )),
        }
    }

    fn read_token(&mut self) -> Token<'a> {
        let (i, token) = next_token(&self.src);
        self.src = i;
        token
    }

    fn peek_token(&mut self) -> Token<'a> {
        let (_, token) = next_token(&self.src);
        token
    }
}
