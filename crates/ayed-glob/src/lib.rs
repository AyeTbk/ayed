use std::path::Path;

#[derive(Debug)]
pub struct Glob {
    nodes: Vec<Node>,
}

impl Glob {
    pub fn new(pattern: &str) -> Self {
        compile(pattern)
    }

    pub fn is_match_path(&self, p: impl AsRef<Path>, is_file: bool) -> bool {
        // this is ugly
        self.is_match_internal(p.as_ref().to_string_lossy().as_ref(), is_file)
    }

    pub fn is_match(&self, s: &str) -> bool {
        self.is_match_internal(s, false)
    }

    fn is_match_internal(&self, s: &str, is_file: bool) -> bool {
        // TODO rework this whole thing to match nodes to `s` directly,
        // instead of trying to split in components.
        // Make it less resilient wrt extraneous/duplicate '/'.
        // Document its expectations.

        // this is ugly too. id rather the algo just works, if possible
        match (&self.nodes[..], s) {
            (&[Node::PathSep], "/") => return !is_file,
            (&[Node::PathSep], "") => return false,
            (&[], "") => return true,
            (&[], "/") => return false,
            (&[Node::Globstar], _) => return true,
            (&[Node::Globstar, Node::PathSep], _) => return !is_file,
            _ => (),
        }

        let components = s
            .split('/')
            .enumerate()
            .filter(|(i, s)| *i == 0 || !s.is_empty())
            .map(|(_, s)| s)
            .collect::<Vec<_>>();
        self.match_components(&components, 0, 0, is_file)
    }

    fn match_components(
        &self,
        components: &[&str],
        i_comp: usize,
        i_node: usize,
        is_file: bool,
    ) -> bool {
        let mut i_comp = i_comp;
        let mut i_node = i_node;
        loop {
            if i_comp >= components.len() {
                break;
            }
            let cur_component = components[i_comp];

            if let Some(Node::Globstar) = self.nodes.get(i_node) {
                for j_comp in i_comp..components.len() {
                    if self.match_components(components, j_comp, i_node + 2, is_file) {
                        return true;
                    }
                }
                return false;
            } else if let Some(next_i_node) = self.match_component(cur_component, i_node) {
                i_node = next_i_node;
                i_comp += 1;

                // Note, because of self.match_component above, next_node is
                // either None or Some(Node::PathSep).
                let next_node = self.nodes.get(i_node);
                let next_node_is_pathsep = matches!(next_node, Some(Node::PathSep));
                let component_was_last = i_comp == components.len();
                let next_node_is_last = i_node + 1 >= self.nodes.len();

                // Check for PathSep between components, and check for terminating PathSep.
                if component_was_last && next_node_is_last {
                    return next_node.is_none() || !is_file;
                } else if next_node_is_pathsep {
                    i_node += 1;
                }
            } else {
                return false;
            }
        }

        let all_nodes_matched = i_node == self.nodes.len();
        all_nodes_matched
    }

    /// Returns the index of the next node (which should be a `PathSep`, if
    /// there is any) if `component` was successfully matched by nodes
    /// starting at `i_node`, or None if `component` didn't match.
    fn match_component(&self, component: &str, i_node: usize) -> Option<usize> {
        let mut component = component;
        let mut i_node = i_node;
        loop {
            if i_node >= self.nodes.len() {
                break;
            }

            let cur_node = &self.nodes[i_node];
            match cur_node {
                Node::Star => {
                    // Note: relies on there being no two Star in a row.
                    let mut compo = component;
                    i_node += 1;
                    let Some(next_node) = self.nodes.get(i_node) else {
                        // No other nodes to match for this component means Star can consume all that remains.
                        return Some(i_node);
                    };
                    'backtracking: loop {
                        if let Some(match_idx) = next_node.look_ahead(compo) {
                            compo = &compo[match_idx..];
                            if let Some(j_node) = self.match_component(compo, i_node) {
                                return Some(j_node);
                            } else {
                                if !compo.is_empty() {
                                    compo = &compo[match_idx + 1..];
                                }
                                continue 'backtracking;
                            }
                        } else {
                            // No amount of chars could be consumed to make next_node match.
                            return None;
                        }
                    }
                }
                Node::PathSep => {
                    if !component.is_empty() {
                        return None;
                    } else {
                        return Some(i_node);
                    }
                }
                Node::Globstar => unreachable!("handled by match_components"),
                _ => {
                    if let Some(rest) = cur_node.consume(component) {
                        component = rest;
                    } else {
                        // Failed to match node; this path component doesn't match the pattern.
                        return None;
                    }
                }
            }

            i_node += 1;
        }

        if !component.is_empty() {
            // Failed to match the whole component.
            return None;
        }

        Some(i_node)
    }
}

#[derive(Debug)]
enum Node {
    Literal { literal: String },
    CharInSet { set: String, invert: bool },
    CountOfAny { count: u32 },
    Star,
    PathSep,
    Globstar,
    Unmatchable,
}

impl Node {
    /// Check against the beginning of `src` and returns the rest of it if it matches.
    fn consume<'a>(&self, src: &'a str) -> Option<&'a str> {
        match self {
            Node::Literal { literal } => src.strip_prefix(literal),
            Node::CharInSet { set, invert } => {
                let Some(ch) = src.chars().next() else {
                    // There wasn't even a char to match against.
                    return None;
                };
                let char_matched = if *invert {
                    set.chars().all(|set_ch| set_ch != ch)
                } else {
                    set.chars().any(|set_ch| set_ch == ch)
                };
                if char_matched {
                    Some(&src[ch.len_utf8()..])
                } else {
                    None
                }
            }
            Node::CountOfAny { count } => {
                let mut chars = src.chars();
                let mut rest_idx = 0;
                for _ in 0..*count {
                    if let Some(ch) = chars.next() {
                        rest_idx += ch.len_utf8();
                    } else {
                        // Failed to match enough characters.
                        return None;
                    }
                }
                Some(&src[rest_idx..])
            }
            Node::Star => None,
            Node::PathSep => None,
            Node::Globstar => None,
            Node::Unmatchable => None,
        }
    }

    /// Find the next position where this node would match.
    fn look_ahead(&self, src: &str) -> Option<usize> {
        match self {
            Node::Literal { literal } => src.find(literal),
            Node::CharInSet { set, invert } => {
                if *invert {
                    src.find(|ch| set.chars().all(|set_ch| set_ch != ch))
                } else {
                    src.find(|ch| set.chars().any(|set_ch| set_ch == ch))
                }
            }
            Node::CountOfAny { count } => {
                if src.chars().count() >= *count as usize {
                    Some(0)
                } else {
                    None
                }
            }
            // This method is used for Star backtracking, and is called when handling
            // Star. Having multiple Star in a row is redundant, so the compiler should
            // just simplify such cases as a single Star, which would make the below
            // case unreachable.
            Node::Star => None,
            Node::PathSep => Some(src.len()),
            Node::Globstar => None,
            Node::Unmatchable => None,
        }
    }
}

fn compile(pattern: &str) -> Glob {
    let mut nodes = Vec::new();

    let mut chars = pattern.chars().peekable();
    let mut buf = String::new();
    let mut invert_class = false;
    let mut run_count = 0;
    let mut range_start_char = None;

    #[derive(PartialEq)]
    enum State {
        FindState,
        // Collapse sequence of '/' into a single PathSep
        PathSeps,
        // Collapse sequence of '*' into a single Star or Globstar
        Stars,
        // Collapse sequence of '?' into a single CountOfAny
        AnyChar,
        // Parse a [...] expression
        CharClass,
        // Maybe parse an escaped ?, * or [
        Escape,
        // Gather up a literal
        Literal,
    }
    let mut state = State::FindState;

    loop {
        let ch;
        let eof;
        if let Some(peek) = chars.peek() {
            ch = *peek;
            eof = false;
        } else {
            ch = '\0';
            eof = true;
        };

        // FIXME support escapes. Outside of `[...]` expressions,
        // the following can be escaped with '\' to be literal
        // characters: '?', '*' and '['.
        match state {
            State::FindState => match ch {
                '/' => state = State::PathSeps,
                '*' => {
                    state = State::Stars;
                    run_count = 1;
                }
                '?' => {
                    state = State::AnyChar;
                    run_count = 1;
                }
                '[' => {
                    // TODO support named classes. Maybe equivalence classes too?
                    state = State::CharClass;
                    invert_class = false;
                    buf.clear();
                }
                _ if eof => { /* Done parsing :) */ }
                _ => {
                    state = State::Literal;
                    buf.clear();
                    continue;
                }
            },
            State::PathSeps => {
                if ch != '/' {
                    nodes.push(Node::PathSep);
                    state = State::FindState;
                    continue;
                }
            }
            State::Stars => match ch {
                '*' => run_count += 1,
                _ => {
                    let node = if run_count == 2 && (ch == '/' || eof) {
                        Node::Globstar
                    } else {
                        Node::Star
                    };
                    nodes.push(node);
                    state = State::FindState;
                    continue;
                }
            },
            State::AnyChar => match ch {
                '?' => run_count += 1,
                _ => {
                    nodes.push(Node::CountOfAny { count: run_count });
                    state = State::FindState;
                    continue;
                }
            },
            State::CharClass => match ch {
                '!' if buf.len() == 0 => {
                    invert_class = true;
                }
                ']' if buf.len() > 0 => {
                    if range_start_char.is_some() {
                        buf.push('-');
                    }
                    nodes.push(Node::CharInSet {
                        set: buf,
                        invert: invert_class,
                    });
                    buf = String::new();
                    state = State::FindState;
                }
                _ if range_start_char.is_some() => {
                    let start_ch = range_start_char.expect("`if` guard ensures this wont fail");
                    for range_ch in (start_ch..=ch).into_iter().skip(1) {
                        buf.push(range_ch);
                    }
                    range_start_char = None;
                }
                '-' if buf.len() > 0 => {
                    range_start_char = buf.chars().last();
                }
                _ if eof => {
                    // Incomplete character class
                    nodes.push(Node::Unmatchable);
                }
                ']' | _ => {
                    buf.push(ch);
                }
            },
            State::Literal => {
                if matches!(ch, '/' | '*' | '?' | '[') || eof {
                    nodes.push(Node::Literal { literal: buf });
                    buf = String::new();
                    state = State::FindState;
                    continue;
                } else if ch == '\\' {
                    state = State::Escape;
                } else {
                    buf.push(ch);
                }
            }
            State::Escape => {
                if !matches!(ch, '?' | '*' | '[') {
                    buf.push('\\');
                }
                buf.push(ch);
                state = State::Literal;
            }
        }

        if eof {
            break;
        } else {
            chars.next();
        }
    }

    // TODO potential improvement: right now it's possible to emit
    // a sequence of multiple Globstars (separated by a PathSep), which
    // should be semantically identical to a single Globstar. Collapse
    // the multiple Globstars. No hurry though, stuff works anyway right
    // now.

    Glob { nodes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        // Good
        assert!(Glob::new("").is_match(""));
        assert!(Glob::new("/").is_match("/"));

        // Bad
        assert!(!Glob::new("").is_match("/"));
        assert!(!Glob::new("/").is_match(""));
    }

    #[test]
    fn escapes_works() {
        // Good
        assert!(Glob::new("\\?").is_match("?"));
        assert!(Glob::new("\\*").is_match("*"));
        assert!(Glob::new("\\[ab]").is_match("[ab]"));
        assert!(Glob::new("\\\\").is_match("\\\\"));
        assert!(Glob::new("[\\[]").is_match("\\"));

        // Bad
        assert!(!Glob::new("\\?").is_match("a"));
        assert!(!Glob::new("\\*").is_match("a"));
        assert!(!Glob::new("\\[ab]").is_match("a"));
        assert!(!Glob::new("\\\\").is_match("\\"));
    }

    #[test]
    fn trailing_pathsep_is_optional_for_is_match() {
        assert!(Glob::new(".").is_match("."));
        assert!(Glob::new(".").is_match("./"));

        assert!(Glob::new("wow").is_match("wow"));
        assert!(Glob::new("wow").is_match("wow/"));

        assert!(Glob::new("wow/amazing/").is_match("wow/amazing"));
        assert!(Glob::new("wow/amazing/").is_match("wow/amazing/"));
    }

    #[test]
    fn any_char_works() {
        // Good
        assert!(Glob::new("?").is_match("a"));
        assert!(Glob::new("???").is_match("abc"));
        assert!(Glob::new("?/?").is_match("a/b"));
        assert!(Glob::new("hell?/w?rld").is_match("hella/warld"));

        // Bad
        assert!(!Glob::new("?").is_match("ab"));
        assert!(!Glob::new("??").is_match("abc"));
    }

    #[test]
    fn basic_character_classes_works() {
        // Good
        assert!(Glob::new("[abc]").is_match("a"));
        assert!(Glob::new("[abc]").is_match("b"));
        assert!(Glob::new("[abc]").is_match("c"));

        // Bad
        assert!(!Glob::new("[abc]").is_match("d"));
    }

    #[test]
    fn negative_character_classes_works() {
        // Good
        assert!(Glob::new("[!abc]").is_match("d"));

        // Bad
        assert!(!Glob::new("[!abc]").is_match("a"));
        assert!(!Glob::new("[!abc]").is_match("b"));
        assert!(!Glob::new("[!abc]").is_match("c"));
    }

    #[test]
    fn character_classes_closing_backet_as_first_char_works() {
        assert!(Glob::new("[]]").is_match("]"));
        assert!(Glob::new("[!]]").is_match("d"));
        assert!(Glob::new("[]!][]!]").is_match("!]"));
        assert!(Glob::new("[]!][]!]").is_match("]!"));
    }

    #[test]
    fn character_classes_ranges_works() {
        // Good
        assert!(Glob::new("[a-d]").is_match("a"));
        assert!(Glob::new("[a-d]").is_match("b"));
        assert!(Glob::new("[a-d]").is_match("c"));
        assert!(Glob::new("[a-d]").is_match("d"));
        assert!(Glob::new("[!a-d]").is_match("e"));

        // Bad
        assert!(!Glob::new("[a-d]").is_match("e"));
        assert!(!Glob::new("[!a-d]").is_match("a"));
    }

    #[test]
    fn character_classes_ranges_dash_is_literal_as_first_or_last_char() {
        // Good
        assert!(Glob::new("[-d]").is_match("-"));
        assert!(Glob::new("[-d]").is_match("d"));
        assert!(Glob::new("[d-]").is_match("-"));
        assert!(Glob::new("[d-]").is_match("d"));

        assert!(Glob::new("[--0]").is_match("-"));
        assert!(Glob::new("[--0]").is_match("."));
        assert!(Glob::new("[--0]").is_match("0"));

        // Bad
        assert!(!Glob::new("[-d]").is_match("a"));
        assert!(!Glob::new("[d-]").is_match("e"));
    }

    #[test]
    fn wildcard_works() {
        // Good
        assert!(Glob::new("*").is_match(""));
        assert!(Glob::new("*").is_match("hehehe"));
        assert!(Glob::new("*").is_match("hihi/"));
        assert!(Glob::new("hu*").is_match("huhu"));
        assert!(Glob::new("ha*/ho*").is_match("haha/hoho"));
        assert!(Glob::new("*/*").is_match("haha/hoho"));
        assert!(Glob::new("ha*").is_match("ha"));
        assert!(Glob::new("ha*/ho*").is_match("ha/ho"));

        // Bad
        assert!(!Glob::new("*").is_match("he/hehehe"));
    }

    #[test]
    fn wildcard_backtracking_works() {
        assert!(Glob::new("*wowa").is_match("wowowa"));
        assert!(Glob::new("*wowa*kwe*kwo").is_match("wowowakwe bleh kwo"));
        assert!(Glob::new("*kwo").is_match("kwokwokwo bleh kwo"));
        assert!(Glob::new("*kwo/*kwe").is_match("kwokwokwo bleh kwo/kwe bleh kwe"));
    }

    #[test]
    fn path_separators_are_collapsed() {
        assert!(Glob::new("///").is_match("/"));
        assert!(Glob::new("///").is_match("//"));
        assert!(Glob::new("///").is_match("///"));
        assert!(Glob::new("///").is_match("////"));
        assert!(Glob::new("///").is_match("/////"));
        assert!(Glob::new("/").is_match("///"));
        assert!(Glob::new("//").is_match("///"));
        assert!(Glob::new("///").is_match("///"));
        assert!(Glob::new("////").is_match("///"));
        assert!(Glob::new("/////").is_match("///"));
        assert!(Glob::new("a//b///c").is_match("a/b/c"));
        assert!(Glob::new("a/b/c").is_match("a//b///c"));
    }

    #[test]
    fn globstar_works() {
        assert!(Glob::new("**").is_match("haha/awd/aas/ebcdic"));
        assert!(Glob::new("**/").is_match("haha/awd/aas/ebcdic"));
        assert!(Glob::new("**").is_match("haha/awd/aas/ebcdic/"));
        assert!(Glob::new("**/").is_match("haha/awd/aas/ebcdic/"));
        assert!(Glob::new("**/ebcdic").is_match("haha/ebcdic"));
        assert!(Glob::new("**/ebcdic").is_match("ebcdic"));
        assert!(Glob::new("**/ebcdic").is_match("haha/awd/aas/ebcdic"));
        assert!(Glob::new("**/**/ebcdic").is_match("haha/awd/aas/ebcdic"));
    }

    #[test]
    fn globstar_works_with_is_file() {
        // See test `is_file_works`
        assert!(Glob::new("**").is_match_path("haha/awd/aas/ebcdic", true));
        assert!(!Glob::new("**/").is_match_path("haha/awd/aas/ebcdic", true));

        assert!(Glob::new("**").is_match_path("haha/awd/aas/ebcdic", false));
        assert!(Glob::new("**/").is_match_path("haha/awd/aas/ebcdic", false));
    }

    #[test]
    fn is_file_works() {
        // A trailing pathsep in the pattern excludes files.
        // Good
        assert!(Glob::new("*").is_match_path("a-file", true));
        assert!(Glob::new("*").is_match_path("a-file/", true));
        // Bad
        assert!(!Glob::new("*/").is_match_path("a-file", true));
        assert!(!Glob::new("*/").is_match_path("a-file/", true));

        // Dirs are unbothered, unchallenged, undefeated.
        assert!(Glob::new("*").is_match_path("a-dir", false));
        assert!(Glob::new("*").is_match_path("a-dir/", false));
        assert!(Glob::new("*/").is_match_path("a-dir", false));
        assert!(Glob::new("*/").is_match_path("a-dir/", false));
    }

    #[test]
    fn should_match_completely_duh() {
        assert!(!Glob::new("/crates/ayed-tui/stderr.txt").is_match_path("/crates/ayed-tui", false));
        assert!(!Glob::new("**/ayed-tui/stderr.txt").is_match_path("/crates/ayed-tui", false));
    }

    // TODO
    // fn named_character_classes_works()
    // fn equivalence_classes_works()
}
