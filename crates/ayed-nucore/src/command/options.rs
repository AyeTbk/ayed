use std::collections::{hash_map::Entry, HashMap, HashSet};

#[derive(Default)]
pub struct Options {
    flags: HashSet<&'static str>,
}

impl Options {
    pub fn new() -> Self {
        Options::default()
    }

    pub fn flag(mut self, flag: &'static str) -> Self {
        self.flags.insert(flag);
        self
    }

    pub fn parse(self, opt_input: &str) -> Result<ParsedOptions, String> {
        // Options are separated by spaces ('\x20').
        // flags: --flag
        // switches: --switch=value (TODO not impl yet)
        let mut opts = ParsedOptions::default();
        for flag in self.flags {
            opts.flags.insert(flag, false);
        }

        let mut i = opt_input.trim_start().trim_end();
        while !i.is_empty() {
            if let Some((rest, flag_name)) = parsers::flag(i) {
                match opts.flags.entry(flag_name) {
                    Entry::Occupied(mut entry) => {
                        entry.insert(true);
                    }
                    _ => {
                        return Err(format!("unknown option: {flag_name}"));
                    }
                }
                i = rest;
                if !i.is_empty() {
                    let (rest, _) = parsers::space_delimiter(i).ok_or_else(|| {
                        "options must be delimited by a single space character".to_string()
                    })?;
                    i = rest;
                }
            } else {
                break;
            }
        }
        opts.remainder = i;

        Ok(opts)
    }
}

#[derive(Default)]
pub struct ParsedOptions<'a> {
    flags: HashMap<&'a str, bool>,
    remainder: &'a str,
}

impl<'a> ParsedOptions<'a> {
    pub fn contains(&self, option_name: &str) -> bool {
        self.flags.get(option_name).copied().unwrap_or_default()
    }

    pub fn remainder(&self) -> &str {
        &self.remainder
    }
}

mod parsers {
    pub fn space_delimiter(i: &str) -> Option<(&str, &str)> {
        take_while1(|c| c == ' ')(i)
    }

    pub fn flag(i: &str) -> Option<(&str, &str)> {
        let (i, _) = tag("--")(i)?;
        let (i, name) = take_while1(char::is_alphabetic)(i)?;
        Some((i, name))
    }

    fn take_while1(pred: impl Fn(char) -> bool) -> impl Fn(&str) -> Option<(&str, &str)> {
        move |i| {
            let mut idx = 0;
            for c in i.chars() {
                if !pred(c) {
                    break;
                }
                idx += c.len_utf8();
            }
            if idx == 0 {
                None
            } else {
                let (parsed, rest) = i.split_at(idx);
                Some((rest, parsed))
            }
        }
    }

    pub fn tag(tag: &'static str) -> impl Fn(&str) -> Option<(&str, &str)> {
        move |i| {
            if i.starts_with(tag) {
                let (parsed, rest) = i.split_at(tag.len());
                Some((rest, parsed))
            } else {
                None
            }
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn test_take_while1() {
            use super::take_while1;

            assert!(matches!(
                take_while1(char::is_alphabetic)("hello2"),
                Some(("2", "hello"))
            ));
            assert!(matches!(take_while1(char::is_alphabetic)("!wow"), None));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_options_parse() {
        use super::Options;

        let opts = Options::new()
            .flag("hello")
            .flag("there")
            .flag("friend")
            .parse("--there --hello yippee!")
            .unwrap();
        assert_eq!(opts.contains("hello"), true);
        assert_eq!(opts.contains("there"), true);
        assert_eq!(opts.contains("friend"), false);
        assert_eq!(opts.contains("pal"), false);
        assert_eq!(opts.remainder(), "yippee!");
    }
}
