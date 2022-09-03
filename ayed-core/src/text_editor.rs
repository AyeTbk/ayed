use crate::{
    buffer::Buffer,
    command::Command,
    core::EditorContextMut,
    input::Input,
    input_mapper::InputMap,
    mode_line::ModeLineInfo,
    panel::Panel,
    selection::{Offset, Position, Selection, Selections},
    text_mode::{TextCommandMode, TextEditMode},
    ui_state::{Color, Span, Style, UiPanel},
};

pub struct TextEditor {
    // TODO active mode sucks right now, make it better.
    // TODO features Id like: execute predefined commands on mode enter / exit,
    active_mode: Box<dyn InputMap>,
    active_mode_name: &'static str,
    active_mode_is_text_edit_append: bool,
    selections: Selections,
    viewport_top_left_position: Position,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            active_mode: Box::new(TextCommandMode),
            active_mode_name: TextCommandMode::NAME,
            active_mode_is_text_edit_append: false,
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
        self.active_mode_is_text_edit_append = false;
        self.active_mode_name = mode_name;
        match mode_name {
            TextCommandMode::NAME => self.active_mode = Box::new(TextCommandMode),
            TextEditMode::NAME => self.active_mode = Box::new(TextEditMode),
            _ => panic!("unsupported mode: {:?}", mode_name),
        }
    }

    pub fn set_mode_with_arg(&mut self, mode_name: &'static str, arg: usize) {
        self.set_mode(mode_name);
        match mode_name {
            TextEditMode::NAME => {
                self.active_mode_is_text_edit_append = arg != 0;
            }
            _ => (),
        }
    }

    fn insert_char(&mut self, ch: char, buffer: &mut Buffer) {
        for idx in 0..self.selections.count() {
            let selection = self
                .selections
                .get(idx)
                .expect("iterating over the selections count");
            self.insert_char_for_selection(ch, selection, buffer);
        }
    }

    fn insert_char_for_selection(&mut self, ch: char, selection: Selection, buffer: &mut Buffer) {
        let insert_at = selection.cursor();
        if let Ok(offset) = buffer.insert_char_at(ch, insert_at) {
            self.move_selections_because_of_insert_char(insert_at, offset);
        } else {
            panic!("tried to insert char outside of buffer")
        }
    }

    fn move_selections_because_of_insert_char(&mut self, inserted_at: Position, offset: Offset) {
        for selection in self.selections.iter_mut() {
            let inserted_before_selection = inserted_at <= selection.start();
            let inserted_after_selection = inserted_at > selection.end();
            let inserted_within_selection = !inserted_before_selection && !inserted_after_selection;

            let mut start = selection.start();
            let mut end = selection.end();

            if inserted_after_selection {
                continue;
            } else if inserted_before_selection {
                // check if same line to move column index
                if inserted_at.line_index == start.line_index {
                    if offset.line_offset != 0 {
                        start.column_index = start.column_index - selection.start().column_index;
                    } else {
                        start.column_index =
                            (start.column_index as i64 + offset.column_offset as i64) as u32;
                    }
                }
                if inserted_at.line_index == end.line_index {
                    if offset.line_offset != 0 {
                        end.column_index = end.column_index - selection.start().column_index
                    } else {
                        end.column_index =
                            (end.column_index as i64 + offset.column_offset as i64) as u32;
                    }
                }

                // need to ajust line index
                start.line_index = (start.line_index as i64 + offset.line_offset as i64) as u32;
                end.line_index = (end.line_index as i64 + offset.line_offset as i64) as u32;
            } else if inserted_within_selection {
                // check if same line as selection.end() to move column index
                if inserted_at.line_index == end.line_index {
                    if offset.line_offset != 0 {
                        end.column_index = 0
                    } else {
                        end.column_index =
                            (end.column_index as i64 + offset.column_offset as i64) as u32;
                    }
                }
                // need to ajust line index
                end.line_index = (end.line_index as i64 + offset.line_offset as i64) as u32;
            }
            let mut new_selection = Selection::new()
                .with_anchor(start)
                .with_cursor(end)
                .flipped_forward();
            if !selection.is_forward() {
                new_selection = new_selection.flipped();
            }
            *selection = new_selection;
        }
    }

    fn delete_before_selection(&mut self, buffer: &mut Buffer) {
        for selection in self.selections.iter_mut() {
            if selection.start() == buffer.start_of_content_position() {
                // Can't delete before the beginning!
                continue;
            }
            let before_selection = buffer
                .moved_position_horizontally(selection.start(), -1)
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

    fn move_cursor_horizontally(
        &mut self,
        column_offset: i32,
        buffer: &Buffer,
        selection_anchored: bool,
    ) {
        for selection in self.selections.iter_mut() {
            let new_position = if let Some(moved_position) =
                buffer.moved_position_horizontally(selection.cursor(), column_offset)
            {
                moved_position
            } else {
                if column_offset < 0 {
                    buffer.start_of_content_position()
                } else {
                    buffer.end_of_content_position()
                }
            };
            *selection = if selection_anchored {
                selection.with_cursor(new_position)
            } else {
                selection.with_position(new_position)
            }
        }
    }

    fn move_cursor_vertically(
        &mut self,
        line_offset: i32,
        buffer: &Buffer,
        selection_anchored: bool,
    ) {
        for selection in self.selections.iter_mut() {
            if let Some(moved_position) =
                buffer.moved_position_vertically(selection.cursor(), line_offset)
            {
                *selection = if selection_anchored {
                    selection.with_cursor(moved_position)
                } else {
                    selection.with_position(moved_position)
                }
            }
        }
    }

    fn move_cursor_to_line_start(&mut self) {
        for selection in self.selections.iter_mut() {
            let new_cursor = selection.cursor().with_column_index(0);
            *selection = selection.with_position(new_cursor);
        }
    }

    fn move_cursor_to_line_end(&mut self, buffer: &Buffer) {
        for selection in self.selections.iter_mut() {
            let line_index = selection.cursor().line_index;
            let line_len = buffer.line_len(line_index).unwrap();
            let new_cursor = selection.cursor().with_column_index(line_len as u32);
            *selection = selection.with_position(new_cursor);
        }
    }

    fn adjust_viewport_to_primary_selection(&mut self, ctx: &EditorContextMut) {
        let mut new_viewport_top_left_position = self.viewport_top_left_position;
        // Horizontal
        let vp_start_x = self.viewport_top_left_position.column_index;
        let vp_after_end_x = vp_start_x + ctx.viewport_size.0;
        let selection_x = self.selections.primary().cursor().column_index;

        if selection_x < vp_start_x {
            new_viewport_top_left_position.column_index = selection_x;
        } else if selection_x >= vp_after_end_x {
            new_viewport_top_left_position.column_index = selection_x - ctx.viewport_size.0 + 1;
        }

        // Vertical
        let vp_start_y = self.viewport_top_left_position.line_index;
        let vp_after_end_y = vp_start_y + ctx.viewport_size.1;
        let selection_y = self.selections.primary().cursor().line_index;

        if selection_y < vp_start_y {
            new_viewport_top_left_position.line_index = selection_y;
        } else if selection_y >= vp_after_end_y {
            new_viewport_top_left_position.line_index = selection_y - ctx.viewport_size.1 + 1;
        }

        self.viewport_top_left_position = new_viewport_top_left_position;
    }

    fn selections(&self) -> impl Iterator<Item = Selection> + '_ {
        self.selections.iter().copied()
    }

    fn selections_split_by_lines<'a>(
        &'a self,
        buffer: &'a Buffer,
    ) -> impl Iterator<Item = (Selection, impl Iterator<Item = Selection> + 'a)> + 'a {
        self.selections().map(move |s| {
            let anchor = s.anchor();
            let cursor = s.cursor();
            (
                s,
                s.line_span().map(move |line_index| {
                    let line_len = buffer
                        .line_len(line_index)
                        .expect("selection spans an invalid line");
                    let line_anchor;
                    let line_cursor;

                    let (cursor_default_column_index, anchor_default_column_index) =
                        if cursor >= anchor {
                            (0, line_len as u32)
                        } else {
                            (line_len as u32, 0)
                        };

                    if line_index == anchor.line_index {
                        line_anchor = anchor;
                    } else {
                        line_anchor = Position::new(line_index, cursor_default_column_index);
                    }
                    if line_index == cursor.line_index {
                        line_cursor = cursor;
                    } else {
                        line_cursor = Position::new(line_index, anchor_default_column_index);
                    }

                    Selection::new()
                        .with_anchor(line_anchor)
                        .with_cursor(line_cursor)
                }),
            )
        })
    }

    fn compute_selection_spans(&self, spans: &mut Vec<Span>, buffer: &Buffer) {
        let selection_color = if self.active_mode_name == TextEditMode::NAME {
            Some(Color::rgb(150, 32, 96))
        } else {
            Some(Color::rgb(18, 72, 139))
        };

        for (selection, selection_split_by_line) in self.selections_split_by_lines(buffer) {
            let cursor_color = if self.active_mode_name == TextEditMode::NAME {
                Some(Color::RED)
            } else {
                Some(Color::WHITE)
            };

            let cursor_from_relative_to_viewport =
                selection.cursor() - self.viewport_top_left_position;
            let cursor_to_relative_to_viewport =
                selection.cursor() - self.viewport_top_left_position;

            spans.push(Span {
                from: cursor_from_relative_to_viewport,
                to: cursor_to_relative_to_viewport,
                style: Style {
                    foreground_color: cursor_color,
                    background_color: None,
                    invert: true,
                },
                importance: 255,
            });

            for line_split_selection in selection_split_by_line {
                if self.viewport_top_left_position.line_index
                    > line_split_selection.start().line_index
                {
                    // If line is before the viewport, ignore it
                    continue;
                }

                let cursor = line_split_selection.cursor();
                let anchor = line_split_selection.anchor();
                let viewport_adjusted_selection = Selection::new()
                    .with_anchor(
                        if self.viewport_top_left_position.column_index > anchor.column_index {
                            anchor.with_column_index(self.viewport_top_left_position.column_index)
                        } else {
                            anchor
                        },
                    )
                    .with_cursor(
                        if self.viewport_top_left_position.column_index > cursor.column_index {
                            cursor.with_column_index(self.viewport_top_left_position.column_index)
                        } else {
                            cursor
                        },
                    );

                let from_relative_to_viewport =
                    viewport_adjusted_selection.start() - self.viewport_top_left_position;
                let to_relative_to_viewport =
                    viewport_adjusted_selection.end() - self.viewport_top_left_position;

                spans.push(Span {
                    from: from_relative_to_viewport,
                    to: to_relative_to_viewport,
                    style: Style {
                        foreground_color: Some(Color::rgb(200, 200, 200)),
                        background_color: selection_color,
                        invert: false,
                    },
                    importance: 254,
                });
            }
        }
    }

    fn for_each_selection(&mut self, func: impl Fn(Selection) -> Selection) {
        for selection in self.selections.iter_mut() {
            *selection = func(*selection);
        }
    }

    fn execute_command_inner(&mut self, command: Command, ctx: &mut EditorContextMut) {
        match command {
            Command::ChangeMode(mode_name) => {
                if self.active_mode_is_text_edit_append {
                    self.execute_command_inner(Command::DragCursorLeft, ctx);
                }
                self.set_mode(mode_name);
            }
            Command::ChangeModeArg(mode_name, arg) => {
                if self.active_mode_is_text_edit_append {
                    self.execute_command_inner(Command::DragCursorLeft, ctx);
                }
                self.set_mode_with_arg(mode_name, arg);
            }
            Command::Insert(c) => self.insert_char(c, ctx.buffer),
            Command::DeleteSelection => self.delete_selection(ctx.buffer),
            Command::DeleteBeforeSelection => self.delete_before_selection(ctx.buffer),

            // Wow
            Command::MoveCursorUp => self.move_cursor_vertically(-1, ctx.buffer, false),
            Command::MoveCursorDown => self.move_cursor_vertically(1, ctx.buffer, false),
            Command::MoveCursorLeft => self.move_cursor_horizontally(-1, ctx.buffer, false),
            Command::MoveCursorRight => self.move_cursor_horizontally(1, ctx.buffer, false),
            //
            Command::DragCursorUp => self.move_cursor_vertically(-1, ctx.buffer, true),
            Command::DragCursorDown => self.move_cursor_vertically(1, ctx.buffer, true),
            Command::DragCursorLeft => self.move_cursor_horizontally(-1, ctx.buffer, true),
            Command::DragCursorRight => self.move_cursor_horizontally(1, ctx.buffer, true),
            //
            Command::MoveCursorToLineStart => self.move_cursor_to_line_start(),
            Command::MoveCursorToLineEnd => self.move_cursor_to_line_end(ctx.buffer),
            //
            Command::FlipSelection => self.for_each_selection(|sel| sel.flipped()),
            Command::FlipSelectionForward => self.for_each_selection(|sel| sel.flipped_forward()),
            Command::FlipSelectionBackward => {
                self.for_each_selection(|sel| sel.flipped_forward().flipped())
            }
        }

        self.adjust_viewport_to_primary_selection(ctx);
    }
}

impl Panel for TextEditor {
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut) -> Vec<Command> {
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
            let mut line_buf = String::new();
            let full_line = if ctx.buffer.copy_line(line_index, &mut line_buf).is_some() {
                line_buf
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

            let mut line = full_line.to_string()[start_column..end_column].to_string();
            let line_visible_part_length = end_column - start_column;
            let padlen = line_slice_max_len as usize - line_visible_part_length;
            line.extend(" ".repeat(padlen).chars());

            panel_content.push(line);
        }

        // Selection spans
        self.compute_selection_spans(&mut panel_spans, &ctx.buffer);

        // Wooowie done
        UiPanel {
            position: (0, 0),
            size: ctx.viewport_size,
            content: panel_content,
            spans: panel_spans,
        }
    }
}
