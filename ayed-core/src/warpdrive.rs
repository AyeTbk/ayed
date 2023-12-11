use std::iter::once;

use crate::{
    command::EditorCommand,
    selection::Selection,
    state::State,
    ui_state::{Color, Span, Style, UiPanel},
    utils::{Offset, Position},
};

// TODO warpdrive improvements
// - highlight whole match with span
// - better key inputs (more localized on the keyboard rather than going through the alphabet)

#[derive(Default)]
pub struct WarpDrive {
    text_content: Vec<String>,
    position_offset: Offset,
    jump_points: Vec<(Vec<char>, Selection)>,
    input: Vec<char>,
}

impl WarpDrive {
    pub fn new(text_content: Vec<String>, position_offset: Offset) -> Option<Self> {
        let jump_points = if false {
            Self::gather_jump_points_prefix(&text_content)
        } else {
            Self::gather_jump_points_alphabet(&text_content)
        };

        if jump_points.is_empty() {
            None
        } else {
            Some(Self {
                text_content,
                position_offset,
                jump_points,
                input: Vec::default(),
            })
        }
    }

    pub fn execute_command(&mut self, command: EditorCommand) -> Option<EditorCommand> {
        self.execute_command_inner(command)
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let mut content = self.text_content.clone();
        let mut spans = Vec::new();

        // Bg - fg
        for (line_index, _) in content.iter().enumerate() {
            let position = Position::ZERO.with_row(line_index as u32);
            spans.push(Span {
                from: position,
                to: position.with_column(state.viewport_size.column as _),
                style: Style {
                    foreground_color: Some(Color::rgb(100, 100, 100)),
                    background_color: Some(Color::rgb(25, 25, 25)),
                    ..Default::default()
                },
                priority: 10,
            });
        }

        for (chars, &selection) in self.jump_points_iter() {
            let position = selection.start();
            let line = content.get_mut(position.row as usize).unwrap();
            let byte_idx = line
                .char_indices()
                .enumerate()
                .filter(|&(char_idx, _)| char_idx == position.column as usize)
                .map(|(_, (byte_idx, _))| byte_idx)
                .next()
                .unwrap();
            for (i, ch) in chars.iter().enumerate() {
                let char_replace_idx = byte_idx + i;
                if char_replace_idx >= line.len() {
                    continue;
                }
                line.remove(byte_idx + i);
                line.insert(byte_idx + i, *ch);
            }

            spans.push(Span {
                from: position,
                to: position.with_moved_indices((chars.len() - 1) as _, 0),
                style: Style {
                    foreground_color: Some(Color::rgb(200, 200, 200)),
                    background_color: Some(Color::rgb(25, 25, 25)),
                    ..Default::default()
                },
                priority: 20,
            });
        }

        UiPanel {
            position: (0, 0).into(),
            size: state.viewport_size,
            content,
            spans,
        }
    }

    fn remove_non_matching_jump_points(&mut self, input: &[char]) {
        let mut remove = Vec::new();
        for (i, (chars, _)) in self.jump_points_iter().enumerate() {
            assert!(input.len() <= chars.len());

            if &chars[..input.len()] != input {
                remove.push(i);
            }
        }

        for i in remove.into_iter().rev() {
            self.jump_points.remove(i);
        }
    }

    fn add_input(&mut self, ch: char) -> Option<Selection> {
        let mut new_input = self.input.clone();
        new_input.push(ch);

        let any_jump_point_matches_new_input = self
            .jump_points_iter()
            .any(|(chars, _)| chars[..new_input.len()] == new_input);
        if !any_jump_point_matches_new_input {
            // It was a bad input that couldn't possibly help narrow down the targeted jump points, so let's ignore it
            return None;
        }

        self.remove_non_matching_jump_points(&new_input);
        self.input = new_input;

        for (chars, position) in self.jump_points_iter() {
            if chars == self.input {
                return Some(*position);
            }
        }
        None
    }

    fn jump_points_iter(&self) -> impl Iterator<Item = (&[char], &Selection)> + '_ {
        self.jump_points.iter().map(|(v, p)| (&v[..], p))
    }

    fn gather_jump_points_prefix(text_content: &[String]) -> Vec<(Vec<char>, Selection)> {
        let words = Self::gather_words(text_content);

        let mut pt = PrefixTree::new();
        for (word, selection) in words {
            pt.add_word(word, selection);
        }
        pt.crunch();

        pt.get_jump_points()
    }

    fn gather_jump_points_alphabet(text_content: &[String]) -> Vec<(Vec<char>, Selection)> {
        fn fill_up_jump_points(
            matches: &[Selection],
            input: &[char],
            mut alphabet: impl Iterator<Item = char> + Clone,
        ) -> Vec<(Vec<char>, Selection)> {
            let make_input = |ch| input.iter().copied().chain(once(ch)).collect::<Vec<char>>();
            let match_per_letter =
                (matches.len() as f32 / alphabet.clone().count() as f32).ceil() as usize;
            if match_per_letter > 1 {
                let mut jump_points = Vec::new();
                let mut alph = alphabet.clone();
                for matches_chunk in matches.chunks(match_per_letter) {
                    let ch = alph
                        .next()
                        .expect("matches were subdivided according to alphabet size, there should have been enough letters");
                    let chars = make_input(ch);
                    let jp = fill_up_jump_points(matches_chunk, &chars, alphabet.clone());
                    jump_points.extend(jp);
                }
                jump_points
            } else {
                let mut jump_points = Vec::new();
                for &selection in matches {
                    let ch = alphabet
                        .next()
                        .expect("there should be enough letter for every match");
                    let chars = make_input(ch);
                    jump_points.push((chars, selection))
                }

                jump_points
            }
        }

        let matches = Self::gather_words(text_content)
            .into_iter()
            .map(|(_, sel)| sel)
            .collect::<Vec<_>>();

        fill_up_jump_points(&matches, &[], 'a'..='z')
    }

    fn gather_words(text_content: &[String]) -> Vec<(&str, Selection)> {
        let re_word = regex::Regex::new(r"\b\w+\b").unwrap();
        let mut words: Vec<(&str, Selection)> = Vec::new();
        for (line_index, line) in text_content.iter().enumerate() {
            for matchh in re_word.find_iter(line) {
                let mut start = None;
                let mut end = None;
                for (column, (byte_idx, _)) in line.char_indices().enumerate() {
                    if matchh.start() == byte_idx {
                        start = Some(Position::new(column as _, line_index as _));
                    }
                    if matchh.end() == byte_idx {
                        end = Some(Position::new((column - 1) as _, line_index as _));
                    }
                }

                let Some(start) = start else { continue };
                let Some(end) = end else { continue };
                let selection = Selection::new().with_anchor(end).with_cursor(start);
                words.push((matchh.as_str(), selection));
            }
        }
        words
    }

    fn execute_command_inner(&mut self, command: EditorCommand) -> Option<EditorCommand> {
        use EditorCommand::*;
        match command {
            Insert(ch) if self.is_jump_point_input(ch) => {
                if let Some(selection) = self.add_input(ch) {
                    let offset_selection = selection
                        .with_cursor(selection.cursor().offset(self.position_offset))
                        .with_anchor(selection.anchor().offset(self.position_offset));
                    Some(SetSelection {
                        cursor: offset_selection.cursor(),
                        anchor: offset_selection.anchor(),
                    })
                } else {
                    None
                }
            }
            _ => Some(Noop),
        }
    }

    fn is_jump_point_input(&self, input: char) -> bool {
        for (chars, _) in &self.jump_points {
            if let Some(ch) = chars.get(self.input.len()) {
                if *ch == input {
                    return true;
                }
            }
        }
        false
    }
}

struct PrefixTree {
    nodes: Vec<PrefixTreeNode>,
    root: usize,
}

impl PrefixTree {
    pub fn new() -> Self {
        let nodes = vec![PrefixTreeNode::default()];
        Self { nodes, root: 0 }
    }

    pub fn add_word(&mut self, word: &str, selection: Selection) {
        let mut current_node_idx = self.root;
        'word_char: for ch in word.chars() {
            // let ch = ch.to_ascii_lowercase();

            // This is up here because of the borrow checker.
            let new_node_idx = self.nodes.len();

            let current_node = self.nodes.get_mut(current_node_idx).unwrap();

            // Search existing children for the next node.
            for &(child_ch, child_idx) in &current_node.children {
                if child_ch == ch {
                    current_node_idx = child_idx;
                    continue 'word_char;
                }
            }

            // Else add a child node for this current letter of the word.
            current_node.children.push((ch, new_node_idx));
            current_node_idx = new_node_idx;
            self.nodes.push(PrefixTreeNode::default());
        }

        // Current node represents the word. Add selection.
        let current_node = self.nodes.get_mut(current_node_idx).unwrap();
        current_node.selections.push(selection);
    }

    fn crunch(&mut self) {
        // Go as deep as possible then walk back up the tree deleting parents
        // while they have no selection and no siblings and bubble up its
        // selections.
        fn crunch_aux(
            this: &mut PrefixTree,
            current_node_idx: usize,
            parent_idx: usize,
            sibling_count: usize,
            parent_selection_count: usize,
        ) -> Vec<Selection> {
            let current_node = this.nodes.get(current_node_idx).unwrap();
            let children = current_node.children.clone();
            let mut children_count = children.len();
            let selection_count = current_node.selections.len();

            let mut children_to_remove = Vec::new();

            if children_count > 0 {
                for (i, (_, child_idx)) in children.into_iter().enumerate() {
                    let bubbled_up_selections = crunch_aux(
                        this,
                        child_idx,
                        current_node_idx,
                        children_count - 1,
                        selection_count,
                    );

                    if !bubbled_up_selections.is_empty() {
                        // Not strictly necessary, but cleaning up makes self.debug_print() output cleaner.
                        children_to_remove.push(i);
                    }

                    let current_node = this.nodes.get_mut(current_node_idx).unwrap();
                    current_node.selections.extend(bubbled_up_selections);
                }

                children_to_remove.reverse();
                children_count -= children_to_remove.len();
                let current_node = this.nodes.get_mut(current_node_idx).unwrap();
                for child in children_to_remove {
                    current_node.children.remove(child);
                }
            }

            if children_count == 0
                && sibling_count == 0
                && parent_selection_count == 0
                && parent_idx != 0
            {
                let current_node = this.nodes.get_mut(current_node_idx).unwrap();
                return std::mem::take(&mut current_node.selections);
            }

            Vec::new()
        }

        crunch_aux(self, 0, 0, usize::MAX, usize::MAX);
    }

    fn get_jump_points(&self) -> Vec<(Vec<char>, Selection)> {
        fn get_jump_points_aux(
            this: &PrefixTree,
            node_idx: usize,
            chars: Vec<char>,
        ) -> Vec<(Vec<char>, Selection)> {
            let current_node = this.nodes.get(node_idx).unwrap();
            let children_count = current_node.children.len();
            let selection_count = current_node.selections.len();

            let mut jump_points = current_node
                .selections
                .iter()
                .enumerate()
                .map(|(i, &sel)| {
                    let mut sel_chars = chars.clone();
                    if children_count > 0 || selection_count > 1 {
                        sel_chars.push('.');
                        if selection_count > 1 {
                            let i_str = (i + 1).to_string();
                            for i_chr in i_str.chars() {
                                sel_chars.push(i_chr);
                            }
                        }
                    }
                    (sel_chars, sel)
                })
                .collect::<Vec<_>>();

            for &(ch, child_idx) in &current_node.children {
                let mut child_chars = chars.clone();
                child_chars.push(ch);

                jump_points.extend(get_jump_points_aux(this, child_idx, child_chars));
            }

            jump_points
        }
        get_jump_points_aux(self, 0, vec![])
    }

    #[allow(unused)]
    fn debug_print(&self) {
        fn debug_print_aux(this: &PrefixTree, node_idx: usize, indent_level: usize) {
            let current_node = this.nodes.get(node_idx).unwrap();
            let indent = " ".repeat(indent_level);
            if !current_node.selections.is_empty() {
                let sel_count = current_node.selections.len();
                eprintln!("{indent}{sel_count} selections");
            }
            for &(ch, child_idx) in &current_node.children {
                eprintln!("{indent}{ch}");
                debug_print_aux(this, child_idx, indent_level + 1);
            }
        }
        eprintln!("== PrefixTree debug print ==");
        debug_print_aux(self, 0, 0);
    }
}

#[derive(Default)]
struct PrefixTreeNode {
    selections: Vec<Selection>,
    children: Vec<(char, usize)>,
}
