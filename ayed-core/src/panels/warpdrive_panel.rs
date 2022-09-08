use std::collections::BTreeMap;

use crate::{
    command::Command,
    core::EditorContextMut,
    input::Input,
    input_mapper::{InputMap, InputMapper},
    panel::Panel,
    selection::{Offset, Position},
    ui_state::{Color, Span, Style, UiPanel},
};

#[derive(Default)]
pub struct WarpDrivePanel {
    text_content: Vec<String>,
    position_offset: Offset,
    jump_points: BTreeMap<char, Position>,
}

impl WarpDrivePanel {
    pub fn new(text_content: Vec<String>, position_offset: Offset) -> Self {
        let jump_points = Self::gather_jump_points(&text_content);
        Self {
            text_content,
            position_offset,
            jump_points,
        }
    }

    fn gather_jump_points(text_content: &[String]) -> BTreeMap<char, Position> {
        let re_word_start = regex::Regex::new(r"\b\w").unwrap();
        let mut chars = ('a'..='z').rev().collect::<Vec<_>>();
        let mut map = BTreeMap::new();
        'iter_lines: for (line_index, line) in text_content.iter().enumerate() {
            for m in re_word_start.find_iter(line) {
                for (column_index, (byte_idx, _)) in line.char_indices().enumerate() {
                    if m.start() == byte_idx {
                        let char_key = if let Some(ch) = chars.pop() {
                            ch
                        } else {
                            break 'iter_lines;
                        };
                        map.insert(char_key, Position::new(line_index as _, column_index as _));
                    }
                }
            }
        }
        map
    }

    fn execute_command_inner(&mut self, command: Command) -> Option<Command> {
        use Command::*;
        match command {
            Insert('\n') => Some(FlipSelection),
            Insert(ch) => {
                if let Some(&position) = self.jump_points.get(&ch) {
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

        for (&ch, &position) in self.jump_points.iter() {
            let line = content.get_mut(position.line_index as usize).unwrap();
            let byte_idx = line
                .char_indices()
                .enumerate()
                .filter(|&(char_idx, _)| char_idx == position.column_index as usize)
                .map(|(_, (byte_idx, _))| byte_idx)
                .next()
                .unwrap();
            line.remove(byte_idx);
            line.insert(byte_idx, ch);

            spans.push(Span {
                from: position,
                to: position,
                style: Style {
                    foreground_color: Some(Color::rgb(200, 200, 200)),
                    background_color: Some(Color::BLUE), //Some(Color::rgb(25, 25, 25)),
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
