use crate::{
    buffer::Buffer,
    command::Command,
    core::EditorContextMut,
    input::Input,
    input_mapper::InputMap,
    mode_line::ModeLineInfo,
    panel::Panel,
    selection::{Position, Selection, SelectionBounds, Selections},
    text_mode::{TextCommandMode, TextEditMode},
    ui_state::{Color, Span, Style, UiPanel},
};

pub struct TextEditor {
    active_mode: Box<dyn InputMap>,
    active_mode_name: &'static str, // TODO make this better, active mode sucks right now
    selections: Selections,
    viewport_top_left_position: Position,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            active_mode: Box::new(TextCommandMode),
            active_mode_name: TextCommandMode::NAME,
            selections: Selections::new(),
            viewport_top_left_position: Position::ZERO,
        }
    }

    pub fn mode_line_infos(&self, ctx: &EditorContextMut) -> Vec<ModeLineInfo> {
        let file_info = if let Some(path) = ctx.buffer.filepath() {
            path.to_string_lossy().into_owned()
        } else {
            "*scratch*".to_string()
        };

        vec![ModeLineInfo {
            text: file_info,
            style: Style::default().with_foreground_color(Color::BLUE),
        }]
    }

    pub fn set_mode(&mut self, mode_name: &'static str) {
        self.active_mode_name = mode_name;
        match mode_name {
            TextCommandMode::NAME => self.active_mode = Box::new(TextCommandMode),
            TextEditMode::NAME => self.active_mode = Box::new(TextEditMode),
            _ => panic!("unsupported mode: {:?}", mode_name),
        }
    }

    pub fn selections(&self) -> impl Iterator<Item = SelectionBounds> + '_ {
        // FIXME this only shows selections as having a length of one
        self.selections.iter().map(|selection| SelectionBounds {
            from: selection.position.with_moved_indices(0, 0),
            to: selection.position.with_moved_indices(0, 1),
        })
    }

    fn insert_char(&mut self, ch: char, buffer: &mut Buffer) {
        for selection in self.selections.iter_mut() {
            if let Ok(new_position) = buffer.insert_char_at(ch, selection.position) {
                selection.position = new_position;
            }
        }
    }

    fn delete_before_selection(&mut self, buffer: &mut Buffer) {
        for selection in self.selections.iter_mut() {
            if selection.position == buffer.start_of_content_position() {
                // Can't delete before the beginning!
                continue;
            }
            let before_selection = buffer
                .moved_position_horizontally(selection.position, -1)
                .expect("wow?");
            buffer.delete_selection(Selection::new().with_position(before_selection));

            let new_selection = selection.with_position(before_selection);
            *selection = new_selection;
        }
    }

    fn delete_selection(&mut self, buffer: &mut Buffer) {
        for selection in self.selections.iter_mut() {
            buffer.delete_selection(*selection);
            *selection = selection.shrunk();
        }
    }

    fn move_selection_horizontally(&mut self, column_offset: i32, buffer: &Buffer) {
        for selection in self.selections.iter_mut() {
            let new_position = if let Some(moved_position) =
                buffer.moved_position_horizontally(selection.position, column_offset)
            {
                moved_position
            } else {
                if column_offset < 0 {
                    buffer.start_of_content_position()
                } else {
                    buffer.end_of_content_position()
                }
            };
            selection.position = new_position;
        }
    }

    fn move_selection_vertically(&mut self, line_offset: i32, buffer: &Buffer) {
        for selection in self.selections.iter_mut() {
            if let Some(moved_position) =
                buffer.moved_position_vertically(selection.position, line_offset)
            {
                selection.position = moved_position;
            }
        }
    }

    fn adjust_viewport_to_primary_selection(&mut self, ctx: &EditorContextMut) {
        let mut new_viewport_top_left_position = self.viewport_top_left_position;
        // Horizontal
        let vp_start_x = self.viewport_top_left_position.column_index;
        let vp_after_end_x = vp_start_x + ctx.viewport_size.0;
        let selection_x = self.selections.primary().position.column_index;

        if selection_x < vp_start_x {
            new_viewport_top_left_position.column_index = selection_x;
        } else if selection_x >= vp_after_end_x {
            new_viewport_top_left_position.column_index = selection_x - ctx.viewport_size.0 + 1;
        }

        // Vertical
        let vp_start_y = self.viewport_top_left_position.line_index;
        let vp_after_end_y = vp_start_y + ctx.viewport_size.1;
        let selection_y = self.selections.primary().position.line_index;

        if selection_y < vp_start_y {
            new_viewport_top_left_position.line_index = selection_y;
        } else if selection_y >= vp_after_end_y {
            new_viewport_top_left_position.line_index = selection_y - ctx.viewport_size.1 + 1;
        }

        self.viewport_top_left_position = new_viewport_top_left_position;
    }

    fn execute_command_inner(&mut self, command: Command, ctx: &mut EditorContextMut) {
        match command {
            Command::ChangeMode(mode_name) => self.set_mode(mode_name),
            Command::Insert(c) => self.insert_char(c, ctx.buffer),
            Command::DeleteBeforeSelection => self.delete_before_selection(ctx.buffer),
            Command::DeleteSelection => self.delete_selection(ctx.buffer),
            Command::MoveSelectionUp => self.move_selection_vertically(-1, ctx.buffer),
            Command::MoveSelectionDown => self.move_selection_vertically(1, ctx.buffer),
            Command::MoveSelectionLeft => self.move_selection_horizontally(-1, ctx.buffer),
            Command::MoveSelectionRight => self.move_selection_horizontally(1, ctx.buffer),
        }

        self.adjust_viewport_to_primary_selection(ctx);
    }
}

impl Panel for TextEditor {
    fn convert_input_to_command(
        &self,
        input: Input,
        ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        self.active_mode.convert_input_to_command(input, ctx)
    }

    fn execute_command(&mut self, command: Command, ctx: &mut EditorContextMut) {
        self.execute_command_inner(command, ctx);
    }

    fn panel(&mut self, ctx: &EditorContextMut) -> UiPanel {
        // Compute content
        let start_line_index = self.viewport_top_left_position.line_index;
        let after_end_line_index = start_line_index + ctx.viewport_size.1;
        let start_column_index = self.viewport_top_left_position.column_index;
        let line_slice_max_len = ctx.viewport_size.0;

        let mut panel_content = Vec::new();
        let mut panel_spans = Vec::new();

        for line_index in start_line_index..after_end_line_index {
            let full_line = if let Some(line) = ctx.buffer.line(line_index) {
                line
            } else {
                let mut non_existant_line = " ".repeat((ctx.viewport_size.0 - 1) as _);
                non_existant_line.insert(0, '~');
                panel_content.push(non_existant_line);
                let line_index_relative_to_viewport = line_index - start_line_index;
                let from =
                    Position::ZERO.with_moved_indices(line_index_relative_to_viewport as _, 0);
                let to = from.with_moved_indices(0, 1);
                panel_spans.push(Span {
                    from,
                    to,
                    style: Style {
                        foreground_color: Some(Color::rgb(155, 100, 200)),
                        background_color: None,
                        invert: false,
                    },
                    importance: !0,
                });
                continue;
            };

            let (start_column, end_column) = if start_column_index as usize >= full_line.len() {
                (0, 0)
            } else {
                let expected_end = start_column_index as usize + line_slice_max_len as usize;
                let end = expected_end.min(full_line.len());
                (start_column_index as usize, end)
            };

            let mut line = full_line[start_column..end_column].to_string();
            let line_visible_part_length = end_column - start_column;
            let padlen = line_slice_max_len as usize - line_visible_part_length;
            line.extend(" ".repeat(padlen).chars());

            panel_content.push(line);
        }

        // Selection spans
        let selection_color = if self.active_mode_name == TextEditMode::NAME {
            Some(Color::RED)
        } else {
            None
        };
        for selection in self.selections() {
            let from_relative_to_viewport = selection.from - self.viewport_top_left_position;
            let to_relative_to_viewport = selection.to - self.viewport_top_left_position;
            panel_spans.push(Span {
                from: from_relative_to_viewport,
                to: to_relative_to_viewport,
                style: Style {
                    foreground_color: selection_color,
                    background_color: None,
                    invert: true,
                },
                importance: !0,
            });
        }

        // Wooowie done
        UiPanel {
            position: (0, 0),
            size: ctx.viewport_size,
            content: panel_content,
            spans: panel_spans,
        }
    }
}
