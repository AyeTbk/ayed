pub fn take_while(s: &str, mut f: impl FnMut(char) -> bool) -> (&str, &str) {
    let mut end = 0;
    for (idx, ch) in s.char_indices() {
        if !f(ch) {
            break;
        }
        end = idx + ch.len_utf8();
    }
    let prefix = &s[..end];
    let rest = &s[end..];
    (prefix, rest)
}

pub fn is_whitespace(c: char) -> bool {
    c.is_ascii_whitespace()
}
