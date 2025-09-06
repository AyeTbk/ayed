use std::{hash::Hash, sync::LazyLock};

use regex::Regex;

static RE_MOD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<([^-]+)-([^>]+)>").unwrap());
static RE_KEY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<([^>]+)>").unwrap());

#[derive(Debug, Clone, Copy, Eq)]
pub struct Input {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl Input {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }.normalized()
    }

    pub fn from_char_mods(ch: char, mut modifiers: Modifiers) -> Self {
        if ch.is_ascii_uppercase() {
            modifiers.shift = true;
        }
        Self::new(Key::Char(ch), modifiers)
    }

    pub fn from_char(ch: char) -> Self {
        Self::from_char_mods(ch, Default::default())
    }

    pub fn char(&self) -> Option<char> {
        let Key::Char(ch) = self.key else {
            return None;
        };

        if self.modifiers.shift() {
            Some(ch.to_ascii_uppercase())
        } else {
            Some(ch)
        }
    }

    fn normalized(self) -> Self {
        match self.key {
            Key::Char(ch) => {
                let key = Key::from_char_normalized(ch);
                let mut modifiers = self.modifiers;
                if ch.is_uppercase() {
                    modifiers.shift = true;
                }
                Self { key, modifiers }
            }
            Key::BackTab => {
                let mut modifiers = self.modifiers;
                modifiers.shift = false;
                Self {
                    key: Key::BackTab,
                    modifiers,
                }
            }
            _ => self,
        }
    }

    pub fn parse(s: &str) -> Result<Input, ()> {
        fn char_group_to_key(src: &str) -> Result<Key, ()> {
            Key::from_string(src).ok_or(())
        }
        fn mod_group_to_modifiers(src: &str) -> Result<Modifiers, ()> {
            let mut modifiers = Modifiers::default();
            for ch in src.chars() {
                match ch {
                    'c' => modifiers = modifiers.with_ctrl(),
                    's' => modifiers = modifiers.with_shift(),
                    'a' => modifiers = modifiers.with_alt(),
                    _ => return Err(()),
                }
            }
            Ok(modifiers)
        }

        let input = if let Some(captures) = RE_MOD.captures(s) {
            // Parse stuff like <ca-k> => ctrl+alt+k
            let modifiers = mod_group_to_modifiers(&captures.get(1).ok_or(())?.as_str())?;
            let key = char_group_to_key(&captures.get(2).ok_or(())?.as_str())?;
            Self::new(key, modifiers)
        } else if let Some(captures) = RE_KEY.captures(s) {
            // Parse stuff like <space> => space
            let key = char_group_to_key(&captures.get(1).ok_or(())?.as_str())?;
            Self::new(key, Default::default())
        } else {
            // TODO when "if let chains" are stable, rewrite this whole else
            // clause into an else if {} else {}
            let Some(ch) = s.chars().next() else {
                return Err(());
            };
            if s.len() == ch.len_utf8() {
                // Parse basic keys without explicit modifiers
                Self::from_char(ch)
            } else {
                return Err(());
            }
        };
        Ok(input)
    }

    pub fn serialize(&self, buf: &mut String) {
        // FIXME make it so it prefers serializing inputs like '<s-a>' to 'A' (only for shift + letter)
        buf.clear();
        if self.key == Key::Char('\0') {
            return;
        }
        if self.modifiers.any() {
            buf.push_str(self.modifiers.as_str());
            buf.push_str("-");
        }
        let is_word = self.key.serialize(buf);
        if is_word || self.modifiers.any() {
            buf.insert_str(0, "<");
            buf.push_str(">");
        }
    }
}

impl Hash for Input {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let norm = self.normalized();
        norm.key.hash(state);
        norm.modifiers.hash(state);
    }
}

impl PartialEq for Input {
    fn eq(&self, other: &Self) -> bool {
        let norm_self = self.normalized();
        let norm_other = other.normalized();
        norm_self.key.eq(&norm_other.key) && norm_self.modifiers.eq(&norm_other.modifiers)
    }
}

impl From<Key> for Input {
    fn from(key: Key) -> Self {
        Self {
            key,
            modifiers: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    BackTab,
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
    Escape,
}

impl Key {
    pub fn from_char_normalized(ch: char) -> Self {
        let normalized_ch = ch.to_lowercase().next().unwrap_or(ch);
        Self::Char(normalized_ch)
    }

    pub fn serialize(&self, buf: &mut String) -> bool {
        // Returns whether the key is serialized as a word (true) or as a singular char (false).
        let s = match self {
            Key::Char(' ') => "space",
            Key::Char('\n') => "ret",
            Key::Char('\t') => "tab",
            Key::BackTab => "backtab",
            Key::Backspace => "backspace",
            Key::Delete => "del",
            Key::Home => "home",
            Key::End => "end",
            Key::Up => "up",
            Key::Down => "down",
            Key::Left => "left",
            Key::Right => "right",
            Key::PageUp => "pageup",
            Key::PageDown => "pagedown",
            Key::Escape => "esc",
            Key::Char('<') => "lt",
            Key::Char('>') => "gt",
            Key::Char(ch) => {
                buf.push(*ch);
                return false;
            }
        };
        buf.push_str(s);
        return true;
    }

    pub fn from_string(src: &str) -> Option<Self> {
        let key = match src {
            "space" => Key::Char(' '),
            "ret" => Key::Char('\n'),
            "tab" => Key::Char('\t'),
            "backtab" => Key::BackTab,
            "backspace" => Key::Backspace,
            "del" => Key::Delete,
            "home" => Key::Home,
            "end" => Key::End,
            "up" => Key::Up,
            "down" => Key::Down,
            "left" => Key::Left,
            "right" => Key::Right,
            "pageup" => Key::PageUp,
            "pagedown" => Key::PageDown,
            "esc" => Key::Escape,
            "lt" => Key::Char('<'),
            "gt" => Key::Char('>'),
            s => {
                let ch = s.chars().next()?;
                if s.len() != ch.len_utf8() {
                    return None;
                }
                Key::Char(ch)
            }
        };
        Some(key)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl Modifiers {
    pub fn ctrl(&self) -> bool {
        self.ctrl
    }

    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn shift(&self) -> bool {
        self.shift
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn alt(&self) -> bool {
        self.alt
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub fn any(&self) -> bool {
        self.ctrl || self.shift || self.alt
    }

    pub fn as_str(&self) -> &'static str {
        match (self.ctrl, self.shift, self.alt) {
            (false, false, false) => "",
            (true, false, false) => "c",
            (false, true, false) => "s",
            (false, false, true) => "a",
            (true, true, false) => "cs",
            (true, false, true) => "ca",
            (false, true, true) => "sa",
            (true, true, true) => "csa",
        }
    }
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = String::new();
        self.serialize(&mut buf);
        f.write_str(&buf)
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_eq__normalized_comparison() {
        let not_uppercase_but_shift_wtf = Input {
            key: Key::Char('s'),
            modifiers: Modifiers::default().with_shift(),
        };
        let uppercase_without_shift = Input {
            key: Key::Char('S'),
            modifiers: Modifiers::default(),
        };
        assert_eq!(not_uppercase_but_shift_wtf, uppercase_without_shift)
    }

    #[test]
    fn try_parse_input__one_modifier_and_uppercase_letter() {
        let result = Input::parse("<c-M>").unwrap();
        assert_eq!(
            result,
            Input::from_char_mods('m', Modifiers::default().with_ctrl().with_shift())
        )
    }

    #[test]
    fn try_parse_input__two_modifiers() {
        let result = Input::parse("<ca-l>").unwrap();
        assert_eq!(
            result,
            Input::from_char_mods('l', Modifiers::default().with_ctrl().with_alt())
        )
    }

    #[test]
    fn try_parse_input__three_modifiers() {
        let result = Input::parse("<sac-space>").unwrap();
        assert_eq!(
            result,
            Input::from_char_mods(
                ' ',
                Modifiers::default().with_shift().with_alt().with_ctrl()
            )
        )
    }

    #[test]
    fn try_parse_input__named_key() {
        let result = Input::parse("<left>").unwrap();
        assert_eq!(
            result,
            Input {
                key: Key::Left,
                modifiers: Modifiers::default()
            }
        )
    }
}
