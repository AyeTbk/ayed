use crate::{ast, AsIterator};

use self::nfa::{build_nfa, run_nfa};

mod nfa;

pub struct Regex {
    nfa: nfa::Automaton,
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Self, String> {
        let ast = ast::parse(pattern)?;
        let nfa = build_nfa(&ast);
        Ok(Self { nfa })
    }

    pub fn is_match(&self, text: impl AsIterator<Item = char>) -> bool {
        run_nfa(&self.nfa, &text.as_iter())
    }
}
