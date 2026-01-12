use crate::{
    Error, ErrorKind,
    ast::{Ast, Block, BlockKind, MappingBlock, MappingEntry, MixinBlock, SelectorBlock, Span, Template},
    error::Expected,
    token::{
        Token, TokenKind, is_whitespace, next_entry_name, next_token, next_token_in_entry_value, next_token_in_string, take_while0
    },
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

        let name = self.expect(TokenKind::Identifier)?;

        let kind = match name.slice {
            "mixin" => BlockKind::Mixin(self.parse_mixin_block()?),
            "use" => {
                let mixin_name = self.expect(TokenKind::Identifier)?.slice;
                BlockKind::Use(Span::from(mixin_name))
            }
            _ => {
                let lookahead = self.peek_token();
                match lookahead.kind {
                    TokenKind::Delimiter => BlockKind::Mapping(self.parse_mapping_block(name)?),
                    _ => BlockKind::Selector(self.parse_selector_block(name)?),
                    // token_kind => {
                    //     return Err(Error::new(
                    //         ErrorKind::Unexpected(token_kind.into()),
                    //         lookahead.slice,
                    //     ));
                    // }
                }
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

    fn parse_selector_block(
        &mut self,
        state_name: Token<'a>,
    ) -> Result<SelectorBlock<'a>, Error<'a>> {
        let pattern = self.parse_pattern()?;
        let children = self.parse_delimited_list(Self::parse_block, "{", "}", None)?;
        Ok(SelectorBlock {
            state_name: state_name.slice.into(),
            pattern: pattern.slice.into(),
            children,
        })
    }

    fn parse_mapping_block(&mut self, name: Token<'a>) -> Result<MappingBlock<'a>, Error<'a>> {
        let entries = self.parse_delimited_list(Self::parse_mapping_entry, "{", "}", None)?;
        Ok(MappingBlock {
            name: name.slice.into(),
            entries,
        })
    }

    fn parse_mixin_block(&mut self) -> Result<MixinBlock<'a>, Error<'a>> {
        let name = self.expect(TokenKind::Identifier)?;
        let children = self.parse_delimited_list(Self::parse_block, "{", "}", None)?;
        Ok(MixinBlock {
            name: name.slice.into(),
            children,
        })
    }

    fn parse_mapping_entry(&mut self) -> Result<MappingEntry<'a>, Error<'a>> {
        let name = self.parse_entry_name()?;
        let values = self.parse_entry_values()?;
        Ok(MappingEntry {
            name: name.slice.into(),
            values,
        })
    }

    fn parse_entry_name(&mut self) -> Result<Span<'a>, Error<'a>> {
        let Some((i, name)) = next_entry_name(self.src) else {
            return Err(Error::new(
                ErrorKind::Unexpected(Expected::TokenKind(TokenKind::EntryName)),
                self.src,
            ));
        };
        self.src = i;
        Ok(name.slice.into())
    }

    fn parse_entry_values(&mut self) -> Result<Vec<Template<'a>>, Error<'a>> {
        let is_list = self.peek_token().slice == "$[";
        if is_list {
            return self.parse_entry_value_list();
        }

        let value = self.parse_entry_value(false)?;
        Ok(vec![value])
    }

    fn parse_entry_value_list(&mut self) -> Result<Vec<Template<'a>>, Error<'a>> {
        let values =
            self.parse_delimited_list(|this| this.parse_entry_value(true), "$[", "]", Some(";"))?;
        Ok(values)
    }

    fn parse_entry_value(&mut self, in_list: bool) -> Result<Template<'a>, Error<'a>> {
        let (mut i, _) = take_while0(is_whitespace)(self.src);
        let mut parts = Vec::new();
        while let Some((j, token)) = next_token_in_entry_value(i, in_list) {
            if token.kind == TokenKind::Escape {
                todo!("properly handle escapes");
            }

            if token.kind == TokenKind::Delimiter && token.slice != "$\"" {
                break;
            }

            i = j;

            if token.slice == "$\"" {
                self.src = i;
                let str_template = self.parse_entry_value_string()?;
                i = self.src;
                parts.extend(str_template.parts);
            } else {
                parts.push(token.slice.into());
            }
        }
        self.src = i;
        if parts.is_empty() {
            return Err(Error::new(
                ErrorKind::Unexpected(Expected::EntryValue),
                self.src,
            ));
        }
        Ok(Template { parts })
    }

    fn parse_entry_value_string(&mut self) -> Result<Template<'a>, Error<'a>> {
        let mut i = self.src;
        let mut parts = Vec::new();
        while let Some((j, token)) = next_token_in_string(i) {
            if token.kind == TokenKind::Escape {
                todo!("properly handle escapes");
            }

            i = j;
            if token.slice == "\"" {
                break;
            }
            parts.push(token.slice.into());
        }
        self.src = i;
        if parts.is_empty() {
            return Err(Error::new(
                ErrorKind::Unexpected(Expected::String),
                self.src,
            ));
        }
        Ok(Template { parts })
    }

    fn parse_pattern(&mut self) -> Result<Span<'a>, Error<'a>> {
        let (j, _) = take_while0(is_whitespace)(self.src);
        let (k, pattern) = take_while0(|c| !is_whitespace(c) && c != '{')(j);
        // let (l, _) = take_while0(is_whitespace)(k); // I think this shouldnt be needed
        self.src = k;
        Ok(Span { slice: pattern })
    }

    fn parse_delimited_list<T>(
        &mut self,
        parse_fn: impl Fn(&mut Self) -> Result<T, Error<'a>>,
        open: &'static str,
        close: &'static str,
        sep: Option<&'static str>,
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

                let mut peek = self.peek_token();
                let mut must_close = false;
                if let Some(sep) = sep {
                    if peek.slice == sep {
                        self.read_token();
                        peek = self.peek_token();
                    } else {
                        must_close = true;
                    }
                }
                if peek.slice == close {
                    break 'looop;
                } else if must_close {
                    return Err(Error::new(
                        ErrorKind::Unexpected(Expected::Tag(close)),
                        self.src,
                    ));
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
