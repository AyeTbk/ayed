#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub slice: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Identifier,
    Delimiter,
    EntryName,
    EntryValue,
    Invalid,
    Eof,
}

pub fn next_token<'a>(mut i: &'a str) -> (&'a str, Token<'a>) {
    // The loop only exist to allow skipping comments.
    loop {
        let (j, _) = take_while0_nofail(is_whitespace)(i);
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
            let (m, _comment) = take_while0_nofail(|ch| ch != '\n')(l);
            i = m;
            continue;
        }

        if let Some((l, delimiter)) = any_of(&["{", "}", "$[", "]"])(j) {
            return (
                l,
                Token {
                    kind: TokenKind::Delimiter,
                    slice: delimiter,
                },
            );
        }

        if let Some((l, identifier)) = take_while1(|ch| is_identifier(ch))(j) {
            return (
                l,
                Token {
                    kind: TokenKind::Identifier,
                    slice: identifier,
                },
            );
        }

        if let Some((l, invalid)) = take_while0(|ch| !is_whitespace(ch))(j) {
            return (
                l,
                Token {
                    kind: TokenKind::Invalid,
                    slice: invalid,
                },
            );
        }

        unreachable!();
    }
}

pub fn next_entry_name<'a>(i: &'a str) -> Option<(&'a str, Token<'a>)> {
    let (i, _) = take_while0_nofail(is_whitespace)(i);
    let (i, value) = take_while1(|ch| !is_whitespace(ch))(i)?;
    Some((
        i,
        Token {
            kind: TokenKind::EntryName,
            slice: value,
        },
    ))
}

pub fn next_entry_value<'a>(i: &'a str) -> (&'a str, Token<'a>) {
    let (i, _) = take_while0_nofail(is_whitespace)(i);
    let (i, value) = take_while0_nofail(|ch| ch != '\n')(i);
    (
        i,
        Token {
            kind: TokenKind::EntryValue,
            slice: value,
        },
    )
}

pub fn next_entry_value_in_list<'a>(i: &'a str) -> (&'a str, Token<'a>) {
    let (i, _) = take_while0_nofail(is_whitespace)(i);
    // Keep grabbing, until whitespace delimited ';' or ']'
    let mut end_idx = None;
    let mut prev_ch_was_whitespace = false;
    let mut check_if_next_ch_is_whitespace = false;
    for (idx, ch) in i.char_indices() {
        if check_if_next_ch_is_whitespace {
            check_if_next_ch_is_whitespace = false;
            let next_ch_is_whitespace = is_whitespace(ch);
            if next_ch_is_whitespace {
                break;
            }
        }
        let curr_ch_is_whitespace = is_whitespace(ch);
        if matches!(ch, ';' | ']') && prev_ch_was_whitespace {
            check_if_next_ch_is_whitespace = true;
        } else {
            if !curr_ch_is_whitespace {
                end_idx = Some(idx + ch.len_utf8());
            }
        }
        prev_ch_was_whitespace = curr_ch_is_whitespace;
    }
    let end_idx = end_idx.unwrap_or(i.len());
    let (j, value) = (&i[end_idx..], &i[..end_idx]);
    (
        j,
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

pub fn take_while0(pred: fn(char) -> bool) -> impl Fn(&str) -> Option<(&str, &str)> {
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

pub fn take_while0_nofail(pred: fn(char) -> bool) -> impl Fn(&str) -> (&str, &str) {
    move |i| take_while0(pred)(i).unwrap_or((i, &i[..0]))
}

pub fn take_while1(pred: fn(char) -> bool) -> impl Fn(&str) -> Option<(&str, &str)> {
    move |i| {
        let (j, o) = take_while0(pred)(i)?;
        if o.is_empty() { None } else { Some((j, o)) }
    }
}

pub fn is_whitespace(ch: char) -> bool {
    ch == ' ' || ch == '\t' || ch == '\n'
}

fn is_identifier(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-')
}
