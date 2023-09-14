use std::iter::once;

use crate::{
    command::EditorCommand,
    input::Input,
    input_mapper::InputMapper,
    selection::{Offset, Position, Selection},
    state::State,
    ui_state::{Color, Span, Style, UiPanel},
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
        let jump_points = Self::gather_jump_points(&text_content);

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

    pub fn convert_input_to_command(&self, input: Input, state: &State) -> Vec<EditorCommand> {
        let mut im = InputMapper::default();
        im.register_char_insert();
        im.convert_input(input, state)
    }

    pub fn execute_command(
        &mut self,
        command: EditorCommand,
        _state: &mut State,
    ) -> Option<EditorCommand> {
        self.execute_command_inner(command)
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let mut content = self.text_content.clone();
        let mut spans = Vec::new();

        // Bg - fg
        for (line_index, _) in content.iter().enumerate() {
            let position = Position::ZERO.with_line_index(line_index as u32);
            spans.push(Span {
                from: position,
                to: position.with_column_index(state.viewport_size.0 as _),
                style: Style {
                    foreground_color: Some(Color::rgb(100, 100, 100)),
                    background_color: Some(Color::rgb(25, 25, 25)),
                    invert: false,
                },
                importance: 10,
            });
        }

        for (chars, &selection) in self.jump_points_iter() {
            let position = selection.start();
            let line = content.get_mut(position.line_index as usize).unwrap();
            let byte_idx = line
                .char_indices()
                .enumerate()
                .filter(|&(char_idx, _)| char_idx == position.column_index as usize)
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
                to: position.with_moved_indices(0, (chars.len() - 1) as _),
                style: Style {
                    foreground_color: Some(Color::rgb(200, 200, 200)),
                    background_color: Some(Color::rgb(25, 25, 25)),
                    invert: false,
                },
                importance: 20,
            });
        }

        UiPanel {
            position: (0, 0),
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

    fn gather_jump_points(text_content: &[String]) -> Vec<(Vec<char>, Selection)> {
        // Get matches
        let re_word_start = regex::Regex::new(r"\b\w+\b").unwrap();
        let mut matches: Vec<Selection> = Vec::new();
        for (line_index, line) in text_content.iter().enumerate() {
            for m in re_word_start.find_iter(line) {
                let mut start = None;
                let mut end = None;
                for (column_index, (byte_idx, _)) in line.char_indices().enumerate() {
                    if m.start() == byte_idx {
                        start = Some(Position::new(line_index as _, column_index as _));
                    }
                    if m.end() == byte_idx {
                        end = Some(Position::new(line_index as _, (column_index - 1) as _));
                    }
                }

                let Some(start) = start else { continue };
                let Some(end) = end else { continue };
                matches.push(Selection::new().with_cursor(start).with_anchor(end));
            }
        }

        // Do thing
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

        fill_up_jump_points(&matches, &[], ('a'..='v').chain('x'..='z')) // skip 'w' because it's the Warpdrive keybind at the moment
    }

    fn execute_command_inner(&mut self, command: EditorCommand) -> Option<EditorCommand> {
        use EditorCommand::*;
        match command {
            Insert('\n') => Some(FlipSelection),
            Insert(ch) => {
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
            _ => None,
        }
    }
}
