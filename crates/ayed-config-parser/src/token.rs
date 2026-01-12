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
    Escape,
    Verbatim,
    Invalid,
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

        if let Some((l, _)) = comment(j) {
            i = l;
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

        if let Some((l, invalid)) = take_while(|ch| !is_whitespace(ch))(j) {
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

pub fn next_token_in_entry_value<'a>(i: &'a str, in_list: bool) -> Option<(&'a str, Token<'a>)> {
    let string_start = tag("$\"");
    let line_terminator = tag("\n");

    if let Some((j, slice)) = string_start(i) {
        return Some((
            j,
            Token {
                kind: TokenKind::Delimiter,
                slice,
            },
        ));
    }
    if let Some((j, slice)) = line_terminator(i) {
        return Some((
            j,
            Token {
                kind: TokenKind::Delimiter,
                slice,
            },
        ));
    }
    if in_list {
        let list_end = whitespace_delimited(tag("]"));
        let list_sep = whitespace_delimited(tag(";"));
        if let Some((j, slice)) = list_end(i) {
            return Some((
                j,
                Token {
                    kind: TokenKind::Delimiter,
                    slice,
                },
            ));
        }
        if let Some((j, slice)) = list_sep(i) {
            return Some((
                j,
                Token {
                    kind: TokenKind::Delimiter,
                    slice,
                },
            ));
        }
    }
    if let Some((j, tok)) = escape_sequence(i) {
        return Some((j, tok));
    }
    next_token_in_entry_value_verbatim(i, in_list)
}

fn next_token_in_entry_value_verbatim<'a>(
    i: &'a str,
    in_list: bool,
) -> Option<(&'a str, Token<'a>)> {
    let mut end_idx = None;
    let mut prev_ch_was_whitespace = false;
    let mut check_if_next_ch_is_whitespace = false;
    for (idx, ch) in i.char_indices() {
        if !in_list && ch == '\n' {
            end_idx = Some(idx);
            break;
        }

        if ch == '$' {
            end_idx = Some(idx);
            break;
        }

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
    if !value.is_empty() {
        Some((
            j,
            Token {
                kind: TokenKind::Verbatim,
                slice: value,
            },
        ))
    } else {
        None
    }
}

pub fn next_token_in_string<'a>(i: &'a str) -> Option<(&'a str, Token<'a>)> {
    let string_end = tag("\"");

    if let Some((j, slice)) = string_end(i) {
        return Some((
            j,
            Token {
                kind: TokenKind::Delimiter,
                slice,
            },
        ));
    }
    if let Some((j, tok)) = escape_sequence(i) {
        return Some((j, tok));
    }
    next_token_in_string_verbatim(i)
}

fn next_token_in_string_verbatim<'a>(i: &'a str) -> Option<(&'a str, Token<'a>)> {
    let mut end_idx = None;
    for (idx, ch) in i.char_indices() {
        if ch == '$' || ch == '"' {
            end_idx = Some(idx);
            break;
        }
    }
    let end_idx = end_idx.unwrap_or(i.len());
    let (j, value) = (&i[end_idx..], &i[..end_idx]);
    if !value.is_empty() {
        Some((
            j,
            Token {
                kind: TokenKind::Verbatim,
                slice: value,
            },
        ))
    } else {
        None
    }
}

pub fn next_entry_name<'a>(i: &'a str) -> Option<(&'a str, Token<'a>)> {
    let mut i = i;
    loop {
        let (j, _) = take_while0(is_whitespace)(i);
        i = j;

        if let Some((j, _)) = comment(i) {
            i = j;
            continue;
        }

        let (j, value) = take_while1(|ch| !is_whitespace(ch))(i)?;
        break Some((
            j,
            Token {
                kind: TokenKind::EntryName,
                slice: value,
            },
        ));
    }
}

fn escape_sequence<'a>(i: &'a str) -> Option<(&'a str, Token<'a>)> {
    let mut chars_indices = i.char_indices();
    let (idx, dollar_sign) = chars_indices.next()?;
    if dollar_sign != '$' {
        return None;
    }
    let mut end_idx = idx + dollar_sign.len_utf8();

    if let Some((idx, ch)) = chars_indices.next() {
        end_idx = idx + ch.len_utf8();
    }

    let (j, slice) = (&i[end_idx..], &i[..end_idx]);
    let token = Token {
        kind: TokenKind::Escape,
        slice,
    };
    Some((j, token))
}

fn comment(i: &str) -> Option<(&str, &str)> {
    let (j, tagstr) = tag("#")(i)?;
    let (k, commentstr) = take_while0(|ch| ch != '\n')(j);
    let len = tagstr.len() + commentstr.len();
    Some((k, &i[..len]))
}

pub fn tag(tag: &'static str) -> impl Fn(&str) -> Option<(&str, &str)> {
    move |i| {
        if i.starts_with(tag) {
            let idx = tag.len();
            Some((&i[idx..], &i[..idx]))
        } else {
            None
        }
    }
}

fn whitespace_delimited(
    f: impl Fn(&str) -> Option<(&str, &str)>,
) -> impl Fn(&str) -> Option<(&str, &str)> {
    move |i| {
        let (j, _) = take_while1(is_whitespace)(i)?;
        let (k, o) = f(j)?;
        let (l, _) = take_while1(is_whitespace)(k)?;
        Some((l, o))
    }
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

pub fn take_while0(pred: fn(char) -> bool) -> impl Fn(&str) -> (&str, &str) {
    move |i| take_while(pred)(i).unwrap_or((i, &i[..0]))
}

pub fn take_while1(pred: fn(char) -> bool) -> impl Fn(&str) -> Option<(&str, &str)> {
    move |i| {
        let (j, o) = take_while(pred)(i)?;
        if o.is_empty() { None } else { Some((j, o)) }
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

pub fn is_whitespace(ch: char) -> bool {
    ch == ' ' || ch == '\t' || ch == '\n'
}

fn is_identifier(ch: char) -> bool {
    matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-')
}
