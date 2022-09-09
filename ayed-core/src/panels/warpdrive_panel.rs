use std::iter::once;

use crate::{
    command::Command,
    core::EditorContextMut,
    input::Input,
    input_mapper::{InputMap, InputMapper},
    panel::Panel,
    selection::{Offset, Position},
    ui_state::{Color, Span, Style, UiPanel},
};

// TODO warpdrive improvements
// - select match instead of just placing selection at start of match
// - better key inputs (more localized on the keyboard rather than going through the alphabet)

#[derive(Default)]
pub struct WarpDrivePanel {
    text_content: Vec<String>,
    position_offset: Offset,
    jump_points: Vec<(Vec<char>, Position)>,
    input: Vec<char>,
}

impl WarpDrivePanel {
    pub fn new(text_content: Vec<String>, position_offset: Offset) -> Self {
        let jump_points = Self::gather_jump_points(&text_content);
        Self {
            text_content,
            position_offset,
            jump_points,
            input: Vec::default(),
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

    fn add_input(&mut self, ch: char) -> Option<Position> {
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

    fn jump_points_iter(&self) -> impl Iterator<Item = (&[char], &Position)> + '_ {
        self.jump_points.iter().map(|(v, p)| (&v[..], p))
    }

    fn gather_jump_points(text_content: &[String]) -> Vec<(Vec<char>, Position)> {
        // Get matches
        let re_word_start = regex::Regex::new(r"\b\w").unwrap();
        let mut matches: Vec<Position> = Vec::new();
        for (line_index, line) in text_content.iter().enumerate() {
            for m in re_word_start.find_iter(line) {
                for (column_index, (byte_idx, _)) in line.char_indices().enumerate() {
                    if m.start() == byte_idx {
                        matches.push(Position::new(line_index as _, column_index as _));
                    }
                }
            }
        }

        // Do thing
        fn fill_up_jump_points(
            matches: &[Position],
            input: &[char],
            mut alphabet: impl Iterator<Item = char> + Clone,
        ) -> Vec<(Vec<char>, Position)> {
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
                for &position in matches {
                    let ch = alphabet
                        .next()
                        .expect("there should be enough letter for every match");
                    let chars = make_input(ch);
                    jump_points.push((chars, position))
                }

                jump_points
            }
        }

        fill_up_jump_points(&matches, &[], 'a'..='z')
    }

    fn execute_command_inner(&mut self, command: Command) -> Option<Command> {
        use Command::*;
        match command {
            Insert('\n') => Some(FlipSelection),
            Insert(ch) => {
                if let Some(position) = self.add_input(ch) {
                    let offset_position = position.with_moved_indices(
                        self.position_offset.line_offset,
                        self.position_offset.column_offset,
                    );
                    Some(SetSelection {
                        cursor: offset_position,
                        anchor: offset_position,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl Panel for WarpDrivePanel {
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut) -> Vec<Command> {
        let mut im = InputMapper::default();
        im.register_char_insert();
        im.convert_input_to_command(input, ctx)
    }

    fn execute_command(
        &mut self,
        command: Command,
        _ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        self.execute_command_inner(command)
    }

    fn panel(&mut self, ctx: &EditorContextMut) -> UiPanel {
        let mut content = self.text_content.clone();
        let mut spans = Vec::new();

        // Bg - fg
        for (line_index, _) in content.iter().enumerate() {
            let position = Position::ZERO.with_line_index(line_index as u32);
            spans.push(Span {
                from: position,
                to: position.with_column_index(ctx.viewport_size.0 as _),
                style: Style {
                    foreground_color: Some(Color::rgb(100, 100, 100)),
                    background_color: Some(Color::rgb(25, 25, 25)),
                    invert: false,
                },
                importance: 10,
            });
        }

        for (chars, &position) in self.jump_points_iter() {
            let line = content.get_mut(position.line_index as usize).unwrap();
            let byte_idx = line
                .char_indices()
                .enumerate()
                .filter(|&(char_idx, _)| char_idx == position.column_index as usize)
                .map(|(_, (byte_idx, _))| byte_idx)
                .next()
                .unwrap();
            for (i, ch) in chars.iter().enumerate() {
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
            size: ctx.viewport_size,
            content,
            spans,
        }
    }
}
