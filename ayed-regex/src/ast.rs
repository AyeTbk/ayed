#[derive(Debug)]
pub struct Ast {
    pub root: Node,
}

#[derive(Debug)]
pub enum Node {
    Nothing,
    Char(char),
    Concat(Vec<Self>),
    Alteratives(Vec<Self>),
    Quantified {
        node: Box<Self>,
        quantifier: Quantifier,
    },
    Group(Group),
}

#[derive(Debug)]
pub struct Quantifier {
    pub min: u16,
    pub max: Option<u16>,
    pub lazy: bool,
}

#[derive(Debug)]
pub struct Group {
    pub node: Box<Node>,
    pub capturing: bool,
    pub name: Option<String>,
}

pub fn parse(pattern: &str) -> Result<Ast, String> {
    Parser {
        src: pattern.chars().peekable(),
    }
    .parse()
}

struct Parser<'a> {
    src: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Parser<'a> {
    pub fn parse(mut self) -> Result<Ast, String> {
        Ok(Ast {
            root: self.parse_alternatives()?,
        })
    }

    fn parse_alternatives(&mut self) -> Result<Node, String> {
        let mut alts = Vec::new();
        let mut expecting_more = false;

        while self.peek_char().is_some() {
            let node = self.parse_concatenation()?;
            alts.push(node);

            expecting_more = false;
            if let Some('|') = self.peek_char() {
                self.read_char();
                expecting_more = true;
            } else {
                break;
            }
        }

        if expecting_more {
            alts.push(Node::Nothing);
        }

        if alts.is_empty() {
            Ok(Node::Nothing)
        } else if alts.len() == 1 {
            Ok(alts
                .pop()
                .expect("len should be > 0, as verified in the 'if' condition"))
        } else {
            Ok(Node::Alteratives(alts))
        }
    }

    fn parse_concatenation(&mut self) -> Result<Node, String> {
        let mut concat = Vec::new();
        while let Some(ch) = self.peek_char() {
            let mut node = match ch {
                ')' | '|' => {
                    break;
                }
                '(' => self.parse_group()?,
                '[' => unimplemented!("character classes"),
                '\\' => unimplemented!("escape sequences"),
                '?' | '*' | '+' | '{' => {
                    return Err("preceding token is not quantifiable".to_string())
                }
                _ => Node::Char(self.expect_char()?),
            };

            if let Some(quantifier) = self.try_parse_quantifier()? {
                node = Node::Quantified {
                    node: Box::new(node),
                    quantifier,
                }
            }

            concat.push(node);
        }

        if concat.is_empty() {
            Ok(Node::Nothing)
        } else if concat.len() == 1 {
            Ok(concat
                .pop()
                .expect("len should be > 0, as verified in the 'if' condition"))
        } else {
            Ok(Node::Concat(concat))
        }
    }

    fn parse_group(&mut self) -> Result<Node, String> {
        self.expect_token('(').expect("caller should uphold this");
        let node = self.parse_alternatives()?;
        self.expect_token(')')?;

        Ok(Node::Group(Group {
            node: Box::new(node),
            capturing: true,
            name: None,
        }))
    }

    fn try_parse_quantifier(&mut self) -> Result<Option<Quantifier>, String> {
        let Some(ch) = self.peek_char() else { return Ok(None) };
        let (min, max) = match ch {
            '?' => (0, Some(1)),
            '*' => (0, None),
            '+' => (1, None),
            '{' => {
                self.read_char();
                let min = self.parse_number()?;
                match self.peek_char() {
                    Some('}') => (min, Some(min)),
                    Some(',') => {
                        self.read_char();
                        if let Some('}') = self.peek_char() {
                            (min, None)
                        } else {
                            let max = self.parse_number()?;
                            self.predict_token('}')?;
                            if max < min {
                                return Err("quantifier range is out of order".to_string());
                            }
                            (min, Some(max))
                        }
                    }
                    _ => return Err("incomplete quantifier".to_string()),
                }
            }
            _ => return Ok(None),
        };
        self.read_char();

        let lazy = if let Some('?') = self.peek_char() {
            self.read_char();
            true
        } else {
            false
        };

        Ok(Some(Quantifier { min, max, lazy }))
    }

    fn parse_number(&mut self) -> Result<u16, String> {
        let mut num: u16 = 0;

        let first_char = self.predict_char()?;
        if !first_char.is_ascii_digit() {
            return Err(format!("expected number, found {first_char}"));
        }

        while let Some(ch) = self.peek_char() {
            match ch {
                '0'..='9' => {
                    let digit_value = ch as u16 - '0' as u16;
                    num = num
                        .checked_mul(10)
                        .and_then(|n| n.checked_add(digit_value))
                        .ok_or_else(|| "number is too large")?;
                    self.read_char();
                }
                _ => break,
            }
        }

        Ok(num)
    }

    fn predict_token(&mut self, token: char) -> Result<char, String> {
        let ch = self
            .predict_char()
            .map_err(|_| format!("expected '{token}', found end of pattern"))?;
        if ch != token {
            Err(format!("expected '{token}', found '{ch}'"))
        } else {
            Ok(token)
        }
    }

    fn predict_char(&mut self) -> Result<char, String> {
        self.peek_char()
            .ok_or_else(|| "unexpected end of pattern".to_string())
    }

    fn expect_token(&mut self, token: char) -> Result<char, String> {
        let ch = self
            .expect_char()
            .map_err(|_| format!("expected '{token}', got end of pattern"))?;
        if ch != token {
            Err(format!("expected '{token}', got '{ch}'"))
        } else {
            Ok(token)
        }
    }

    fn expect_char(&mut self) -> Result<char, String> {
        self.read_char()
            .ok_or_else(|| "unexpected end of pattern".to_string())
    }

    fn read_char(&mut self) -> Option<char> {
        self.src.next()
    }

    fn peek_char(&mut self) -> Option<char> {
        self.src.peek().copied()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_parse() {
//         dbg!(parse("(?:wow)"));
//         assert!(false);
//     }
// }
