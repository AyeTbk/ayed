use crate::AsIterator;

use self::nfa::build_nfa;

mod nfa;

pub struct Regex {
    //
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Self, String> {
        build_nfa(pattern)?;
        Ok(Self {})
    }

    pub fn is_match(&self, text: impl AsIterator<Item = char>) -> bool {
        for ch in text.as_iter() {
            print!("{}", ch);
        }
        println!();
        false
    }
}
