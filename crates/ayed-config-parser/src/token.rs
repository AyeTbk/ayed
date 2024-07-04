#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub slice: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    CharSoup,
    Delimiter,
    EntryValue,
    Eof,
}

pub fn next_token<'a>(mut i: &'a str) -> (&'a str, Token<'a>) {
    // The loop only exist to allow skipping comments.
    loop {
        let (j, _) = take_while0(is_whitespace)(i);
        if j.is_empty() {
            return (
                j,
                Token {
                    kind: TokenKind::Eof,
                    slice: j,
                },
            );
        }

        if let Some((l, _)) = any_of(&["#"])(j) {
            let (m, _comment) = take_while0(|ch| ch != '\n')(l);
            i = m;
            continue;
        }

        if let Some((l, delimiter)) = any_of(&["{", "}"])(j) {
            return (
                l,
                Token {
                    kind: TokenKind::Delimiter,
                    slice: delimiter,
                },
            );
        }

        if let Some((l, soup)) = take_while(|ch| !is_whitespace(ch))(j) {
            return (
                l,
                Token {
                    kind: TokenKind::CharSoup,
                    slice: soup,
                },
            );
        }

        unreachable!();
    }
}

pub fn next_entry_value<'a>(i: &'a str) -> (&'a str, Token<'a>) {
    let (i, _) = take_while0(is_whitespace)(i);
    let (i, value) = take_while0(|ch| ch != '\n')(i);
    (
        i,
        Token {
            kind: TokenKind::EntryValue,
            slice: value,
        },
    )
}

fn any_of(tags: &'static [&'static str]) -> impl Fn(&str) -> Option<(&str, &str)> {
    move |i| {
        for tag in tags {
            if i.starts_with(tag) {
                let idx = tag.len();
                return Some((&i[idx..], &i[..idx]));
            }
        }
        None
    }
}

fn take_while(pred: fn(char) -> bool) -> impl Fn(&str) -> Option<(&str, &str)> {
    move |i| {
        let mut end_idx = None;
        for (idx, ch) in i.char_indices() {
            if !pred(ch) {
                end_idx = Some(idx);
                break;
            }
        }
        let end_idx = end_idx.unwrap_or(i.len());
        Some((&i[end_idx..], &i[..end_idx]))
    }
}

fn take_while0(pred: fn(char) -> bool) -> impl Fn(&str) -> (&str, &str) {
    move |i| take_while(pred)(i).unwrap_or((i, &i[..0]))
}

fn is_whitespace(ch: char) -> bool {
    ch == ' ' || ch == '\t' || ch == '\n'
}
