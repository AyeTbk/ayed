#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Input {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl Input {
    pub fn from_char_mod(ch: char, mut modifiers: Modifiers) -> Self {
        let (key, norm_modifiers) = Key::from_char_normalized(ch);
        modifiers.ctrl = modifiers.ctrl || norm_modifiers.ctrl;
        modifiers.shift = modifiers.shift || norm_modifiers.shift;
        modifiers.alt = modifiers.alt || norm_modifiers.alt;
        Self { key, modifiers }
    }

    pub fn from_char(ch: char) -> Self {
        let (key, modifiers) = Key::from_char_normalized(ch);
        Self { key, modifiers }
    }

    pub fn normalized(self) -> Self {
        if let Key::Char(ch) = self.key {
            Self::from_char_mod(ch, self.modifiers)
        } else {
            self
        }
    }

    pub fn char(&self) -> Option<char> {
        match self.key {
            Key::Char(ch) => Some(ch),
            _ => None,
        }
    }

    pub fn try_parse(s: &str) -> Result<Input, ()> {
        fn char_group_to_key(src: &str) -> Result<Key, ()> {
            Ok(match src {
                "space" => Key::Char(' '),
                "tab" => Key::Char('\t'),
                "ret" => Key::Return,
                "backspace" => Key::Backspace,
                "del" => Key::Delete,
                "up" => Key::Up,
                "down" => Key::Down,
                "left" => Key::Left,
                "right" => Key::Right,
                s => {
                    if s.len() != 1 {
                        return Err(());
                    }
                    let ch = s.chars().next().unwrap();
                    Key::Char(ch)
                }
            })
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

        let re_mod = regex::Regex::new(r"<([^-]+)-([^>]+)>").unwrap();
        let re_key = regex::Regex::new(r"<([^>]+)>").unwrap();

        if let Some(captures) = re_mod.captures(s) {
            // Parse stuff like <ca-k> => ctrl+alt+k
            let modifiers = mod_group_to_modifiers(&captures.get(1).ok_or(())?.as_str())?;
            let key = char_group_to_key(&captures.get(2).ok_or(())?.as_str())?;
            let input = Self { key, modifiers }.normalized();
            Ok(input)
        } else if let Some(captures) = re_key.captures(s) {
            // Parse stuff like <space> => space duh
            let key = char_group_to_key(&captures.get(1).ok_or(())?.as_str())?;
            let input = Self {
                key,
                modifiers: Default::default(),
            }
            .normalized();
            Ok(input)
        } else if s.len() == 1 {
            // Parse basic keys without explicit modifiers
            let ch = s.chars().next().ok_or(())?;
            let input = Input::from_char(ch);
            Ok(input)
        } else {
            Err(())
        }
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
    Return,
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
}

impl Key {
    pub fn from_char_normalized(ch: char) -> (Self, Modifiers) {
        let mut mods = Modifiers::default();
        let normalized_ch = ch.to_lowercase().next().unwrap();
        if ch != normalized_ch {
            mods.shift = true;
        }
        (Self::Char(normalized_ch), mods)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl Modifiers {
    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_input__one_modifier_and_uppercase_letter() {
        let result = Input::try_parse("<c-M>").unwrap();
        assert_eq!(
            result,
            Input::from_char_mod('m', Modifiers::default().with_ctrl().with_shift())
        )
    }

    #[test]
    fn try_parse_input__two_modifiers() {
        let result = Input::try_parse("<ca-l>").unwrap();
        assert_eq!(
            result,
            Input::from_char_mod('l', Modifiers::default().with_ctrl().with_alt())
        )
    }

    #[test]
    fn try_parse_input__three_modifiers() {
        let result = Input::try_parse("<sac-space>").unwrap();
        assert_eq!(
            result,
            Input::from_char_mod(
                ' ',
                Modifiers::default().with_shift().with_alt().with_ctrl()
            )
        )
    }

    #[test]
    fn try_parse_input__named_key() {
        let result = Input::try_parse("<left>").unwrap();
        assert_eq!(
            result,
            Input {
                key: Key::Left,
                modifiers: Modifiers::default()
            }
        )
    }
}
