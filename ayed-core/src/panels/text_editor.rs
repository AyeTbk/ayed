use crate::{
    buffer::TextBuffer,
    command::Command,
    input::Input,
    input_mapper::InputMap,
    mode_line::ModeLineInfo,
    panel::Panel,
    selection::{DeletedEditInfo, EditInfo, Position, Selection, Selections},
    state::State,
    text_mode::{TextCommandMode, TextEditMode},
    ui_state::{Color, Span, Style, UiPanel},
};

pub struct TextEditor {
    // TODO active mode sucks right now, make it better.
    // TODO features Id like: execute predefined commands on mode enter / exit,
    active_mode: Box<dyn InputMap>,
    active_mode_name: &'static str,
    selections: Selections,
    view_top_left_position: Position,
    anchor_down: bool,
    anchor_next: AnchorNextState,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            active_mode: Box::new(TextCommandMode),
            active_mode_name: TextCommandMode::NAME,
            selections: Selections::new(),
            view_top_left_position: Position::ZERO,
            anchor_down: false,
            anchor_next: AnchorNextState::Unset,
        }
    }

    pub fn is_command_mode(&self) -> bool {
        self.active_mode_name == TextCommandMode::NAME
    }

    pub fn mode_line_infos(&self, state: &State) -> Vec<ModeLineInfo> {
        let file_info = if let Some(path) = state.active_buffer().filepath() {
            path.to_string_lossy().into_owned()
        } else {
            "*scratch*".to_string()
        };

        vec![ModeLineInfo {
            text: file_info,
            style: Style::default().with_foreground_color(Color::BLUE),
        }]
    }

    pub fn view_top_left_position(&self) -> Position {
        self.view_top_left_position
    }

    pub fn set_mode(&mut self, mode_name: &'static str) {
        self.active_mode_name = mode_name;
        match mode_name {
            TextCommandMode::NAME => self.active_mode = Box::new(TextCommandMode),
            TextEditMode::NAME => self.active_mode = Box::new(TextEditMode),
            _ => panic!("unsupported mode: {:?}", mode_name),
        }
    }

    fn anchored(&self) -> bool {
        self.anchor_down
    }

    fn lower_anchor(&mut self, anchor_next: bool) {
        self.anchor_down = true;
        if anchor_next {
            self.anchor_next = AnchorNextState::JustSet;
        }
    }

    fn raise_anchor(&mut self) {
        self.anchor_down = false;
    }

    fn anchor_check(&mut self) {
        if let AnchorNextState::JustSet = self.anchor_next {
            self.anchor_next = AnchorNextState::Set
        } else if let AnchorNextState::Set = self.anchor_next {
            self.anchor_next = AnchorNextState::Unset;
            self.anchor_down = false;
        }
    }

    fn insert_char_for_each_selection(&mut self, ch: char, buffer: &mut TextBuffer) {
        self.for_each_selection(|this, _, selection| this.insert_char(ch, selection, buffer));
    }

    fn insert_char(&mut self, ch: char, selection: Selection, buffer: &mut TextBuffer) {
        let insert_at = selection.cursor();
        if let Ok(edit) = buffer.insert_char_at(ch, insert_at) {
            self.adjust_selections_from_edit(edit, self.anchored());
        } else {
            panic!("tried to insert char outside of buffer {:?}", insert_at);
        }
    }

    fn delete_selection_for_each_selection(&mut self, buffer: &mut TextBuffer) {
        self.for_each_selection(|this, _, selection| this.delete_selection(selection, buffer));
    }

    fn delete_selection(&mut self, selection: Selection, buffer: &mut TextBuffer) {
        let maybe_edit = buffer.delete_selection(selection).ok();
        if let Some(edit) = maybe_edit {
            self.adjust_selections_from_edit(edit.into(), self.anchored());
        }
    }

    fn delete_cursor_for_each_selection(&mut self, buffer: &mut TextBuffer) {
        self.for_each_selection(|this, _, selection| this.delete_cursor(selection, buffer));
    }

    fn delete_cursor(&mut self, selection: Selection, buffer: &mut TextBuffer) {
        let edit = buffer
            .delete_selection(selection.shrunk_to_cursor())
            .unwrap();
        self.adjust_selections_from_edit(edit.into(), self.anchored());
    }

    fn delete_before_cursor_for_each_selection(&mut self, buffer: &mut TextBuffer) {
        self.for_each_selection(|this, _, selection| {
            if selection.cursor() == Position::ZERO {
                return;
            }
            if let Some(before) = buffer.moved_position_horizontally(selection.cursor(), -1) {
                this.delete_cursor(Selection::new().with_position(before), buffer);
            }
        });
    }

    fn adjust_selections_from_edit(&mut self, edit: EditInfo, selection_anchored: bool) {
        fn adjust_position_from_edit(position: Position, edit: EditInfo) -> Position {
            match edit {
                EditInfo::AddedOne(edit_pos) => {
                    if edit_pos <= position && edit_pos.line_index == position.line_index {
                        position.with_moved_indices(0, 1)
                    } else {
                        position
                    }
                }
                EditInfo::LineSplit(edit_pos) => {
                    if edit_pos <= position {
                        if edit_pos.line_index == position.line_index {
                            let column_distance_from_edit =
                                position.column_index - edit_pos.column_index;
                            Position::new(edit_pos.line_index + 1, column_distance_from_edit)
                        } else {
                            // then position is on a line after the edit
                            position.with_moved_indices(1, 0)
                        }
                    } else {
                        position
                    }
                }
                EditInfo::Deleted(DeletedEditInfo {
                    pos1_line_index,
                    pos1_before_delete_start_column_index,
                    pos2,
                }) => {
                    if position.line_index > pos1_line_index
                        || (position.line_index == pos1_line_index
                            && (position.column_index as i64)
                                >= pos1_before_delete_start_column_index)
                    {
                        let column_index = pos1_before_delete_start_column_index + 1;
                        let pos2_new = Position::new(pos1_line_index, column_index as u32);

                        // If position within edit, place at edit_pos1 + 1column
                        if position <= pos2 {
                            pos2_new
                        } else
                        // If position after edit, place relative to edit_pos2's new position
                        {
                            let delta = pos2_new.offset_between(&pos2);

                            let line_offset = delta.line_offset;
                            let column_offset = if position.line_index == pos2.line_index {
                                delta.column_offset
                            } else {
                                0
                            };

                            position.with_moved_indices(line_offset, column_offset)
                        }
                    } else
                    // If position before edit, do nothing
                    {
                        position
                    }
                }
            }
        }

        for sel in self.selections.iter_mut() {
            let anchor = if selection_anchored && sel.anchor() == edit.pos() {
                sel.anchor()
            } else {
                adjust_position_from_edit(sel.anchor(), edit)
            };
            let cursor = adjust_position_from_edit(sel.cursor(), edit);
            *sel = sel.with_anchor(anchor).with_cursor(cursor);
        }
    }

    fn move_cursor_horizontally(
        &mut self,
        column_offset: i32,
        buffer: &TextBuffer,
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
        buffer: &TextBuffer,
        selection_anchored: bool,
    ) {
        for selection in self.selections.iter_mut() {
            match buffer.moved_position_vertically(selection.desired_cursor(), line_offset) {
                Ok(position) => {
                    *selection = if selection_anchored {
                        selection.with_cursor(position)
                    } else {
                        selection.with_position(position)
                    };
                }
                Err(provisional_position) => {
                    *selection = if selection_anchored {
                        selection.with_provisional_cursor(provisional_position)
                    } else {
                        selection
                            .with_provisional_cursor(provisional_position)
                            .with_anchor(provisional_position)
                    };
                }
            }
        }
    }

    fn move_cursor_to_line_start(&mut self, buffer: &TextBuffer, selection_anchored: bool) {
        for selection in self.selections.iter_mut() {
            let line_index = selection.cursor().line_index;
            let line = buffer
                .line(line_index)
                .expect("the line should exist if a selection is on it");

            let first_non_white_char_index =
                line.find(|ch| !char::is_whitespace(ch)).unwrap_or(0) as u32;

            let new_column_index = if selection.cursor().column_index != first_non_white_char_index
            {
                first_non_white_char_index
            } else {
                0
            };

            let new_cursor = selection.cursor().with_column_index(new_column_index);

            *selection = if selection_anchored {
                selection.with_cursor(new_cursor)
            } else {
                selection.with_position(new_cursor)
            };
        }
    }

    fn move_cursor_to_line_end(&mut self, buffer: &TextBuffer, selection_anchored: bool) {
        for selection in self.selections.iter_mut() {
            let line_index = selection.cursor().line_index;
            let line_len = buffer.line_len(line_index).unwrap();

            // Flip flop between before EOL and at EOL
            let eol_column_index = line_len as u32;
            let last_char_column_index = (eol_column_index).saturating_sub(1);
            let new_column_index = if selection.cursor().column_index != last_char_column_index {
                last_char_column_index
            } else {
                eol_column_index
            };

            let new_cursor = selection.cursor().with_column_index(new_column_index);
            *selection = if selection_anchored {
                selection.with_cursor(new_cursor)
            } else {
                selection.with_position(new_cursor)
            };
        }
    }

    fn duplicate_selection_above_or_below(&mut self, buffer: &TextBuffer, above: bool) {
        let mut new_selections = Vec::new();
        for selection in self.selections() {
            let selection_line_count: i32 = selection.line_span().count().try_into().unwrap();
            let line_offset = if above { -1 } else { 1 } * selection_line_count;
            let dupe_cursor = selection.cursor().with_moved_indices(line_offset, 0);
            let dupe_anchor = selection.anchor().with_moved_indices(line_offset, 0);
            let dupe = Selection::new()
                .with_cursor(dupe_cursor)
                .with_anchor(dupe_anchor);
            let correct_dupe = buffer.limit_selection_to_content(&dupe);
            new_selections.push(correct_dupe);
        }

        for selection in new_selections {
            self.selections.add(selection);
        }
    }

    fn normalize_selections(&mut self) {
        // TODO should I add stuff like limit_selection_to_content and reordering selections?
        self.merge_overlapping_selections();
    }

    fn merge_overlapping_selections(&mut self) {
        self.selections = self.selections.overlapping_selections_merged();
    }

    fn selections(&self) -> impl Iterator<Item = Selection> + '_ {
        self.selections.iter().copied()
    }

    fn selections_split_by_lines<'a>(
        &'a self,
        buffer: &'a TextBuffer,
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

    fn compute_selection_spans(&self, spans: &mut Vec<Span>, buffer: &TextBuffer) {
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

            let cursor_from_relative_to_viewport = selection.cursor() - self.view_top_left_position;
            let cursor_to_relative_to_viewport = selection.cursor() - self.view_top_left_position;

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
                if self.view_top_left_position.line_index > line_split_selection.start().line_index
                {
                    // If line is before the viewport, ignore it
                    continue;
                }

                let cursor = line_split_selection.cursor();
                let anchor = line_split_selection.anchor();
                let viewport_adjusted_selection = Selection::new()
                    .with_anchor(
                        if self.view_top_left_position.column_index > anchor.column_index {
                            anchor.with_column_index(self.view_top_left_position.column_index)
                        } else {
                            anchor
                        },
                    )
                    .with_cursor(
                        if self.view_top_left_position.column_index > cursor.column_index {
                            cursor.with_column_index(self.view_top_left_position.column_index)
                        } else {
                            cursor
                        },
                    );

                let from_relative_to_viewport =
                    viewport_adjusted_selection.start() - self.view_top_left_position;
                let to_relative_to_viewport =
                    viewport_adjusted_selection.end() - self.view_top_left_position;

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

    fn adjust_viewport_to_primary_selection(&mut self, state: &State) {
        let mut new_viewport_top_left_position = self.view_top_left_position;
        // Horizontal
        let vp_start_x = self.view_top_left_position.column_index;
        let vp_after_end_x = vp_start_x + state.viewport_size.0;
        let selection_x = self.selections.primary().cursor().column_index;

        if selection_x < vp_start_x {
            new_viewport_top_left_position.column_index = selection_x;
        } else if selection_x >= vp_after_end_x {
            new_viewport_top_left_position.column_index = selection_x - state.viewport_size.0 + 1;
        }

        // Vertical
        let vp_start_y = self.view_top_left_position.line_index;
        let vp_after_end_y = vp_start_y + state.viewport_size.1;
        let selection_y = self.selections.primary().cursor().line_index;

        if selection_y < vp_start_y {
            new_viewport_top_left_position.line_index = selection_y;
        } else if selection_y >= vp_after_end_y {
            new_viewport_top_left_position.line_index = selection_y - state.viewport_size.1 + 1;
        }

        self.view_top_left_position = new_viewport_top_left_position;
    }

    fn for_each_selection(&mut self, mut func: impl FnMut(&mut Self, usize, Selection)) {
        for idx in 0..self.selections.count() {
            let selection = self
                .selections
                .get(idx)
                .expect("iterating over the selections count");
            func(self, idx, selection);
        }
    }

    fn map_selections(&mut self, func: impl Fn(Selection) -> Selection) {
        for selection in self.selections.iter_mut() {
            *selection = func(*selection);
        }
    }

    fn execute_command_inner(&mut self, command: Command, state: &mut State) {
        let active_buffer = state.active_buffer_mut();

        use Command::*;
        match command {
            AnchorNext => self.lower_anchor(true),
            AnchorDown => self.lower_anchor(false),
            AnchorUp => self.raise_anchor(),

            ChangeMode(mode_name) => {
                self.set_mode(mode_name);
                self.raise_anchor();
            }
            Insert(ch) => self.insert_char_for_each_selection(ch, active_buffer),
            DeleteSelection => self.delete_selection_for_each_selection(active_buffer),
            DeleteCursor => self.delete_cursor_for_each_selection(active_buffer),
            DeleteBeforeCursor => self.delete_before_cursor_for_each_selection(active_buffer),

            // Wow
            MoveCursorUp => self.move_cursor_vertically(-1, active_buffer, self.anchored()),
            MoveCursorDown => self.move_cursor_vertically(1, active_buffer, self.anchored()),
            MoveCursorLeft => self.move_cursor_horizontally(-1, active_buffer, self.anchored()),
            MoveCursorRight => self.move_cursor_horizontally(1, active_buffer, self.anchored()),
            //
            MoveCursorTo(_, _) => todo!(),
            SetSelection { cursor, anchor } => {
                let selection = Selection::new().with_cursor(cursor).with_anchor(anchor);
                self.selections = Selections::new_with(selection, &[]);
            }
            //
            MoveCursorToLineStart => self.move_cursor_to_line_start(active_buffer, self.anchored()),
            MoveCursorToLineEnd => self.move_cursor_to_line_end(active_buffer, self.anchored()),
            //
            ShrinkSelectionToCursor => self.map_selections(|sel| sel.shrunk_to_cursor()),
            FlipSelection => self.map_selections(|sel| sel.flipped()),
            FlipSelectionForward => self.map_selections(|sel| sel.flipped_forward()),
            FlipSelectionBackward => self.map_selections(|sel| sel.flipped_forward().flipped()),
            //
            DuplicateSelectionAbove => self.duplicate_selection_above_or_below(active_buffer, true),
            DuplicateSelectionBelow => {
                self.duplicate_selection_above_or_below(active_buffer, false)
            }
        }

        self.anchor_check();

        self.normalize_selections();

        self.adjust_viewport_to_primary_selection(state);
    }
}

impl Panel for TextEditor {
    fn convert_input_to_command(&self, input: Input, state: &State) -> Vec<Command> {
        self.active_mode.convert_input_to_command(input, state)
    }

    fn execute_command(&mut self, command: Command, state: &mut State) -> Option<Command> {
        self.execute_command_inner(command, state);
        None
    }

    fn render(&mut self, state: &State) -> UiPanel {
        let viewport_size = state.viewport_size;
        let active_buffer = state.active_buffer();

        if viewport_size.0 == 0 || viewport_size.1 == 0 {
            return UiPanel {
                position: (0, 0),
                size: viewport_size,
                content: Default::default(),
                spans: Default::default(),
            };
        }

        self.adjust_viewport_to_primary_selection(state); // this is here to keep the cursor in view when resizing the window

        // Compute content
        let start_line_index = self.view_top_left_position.line_index;
        let after_end_line_index = start_line_index + viewport_size.1;
        let start_column_index = self.view_top_left_position.column_index;
        let line_slice_max_len = viewport_size.0;

        let mut panel_content = Vec::new();
        let mut panel_spans = Vec::new();

        for line_index in start_line_index..after_end_line_index {
            let mut line_buf = String::new();
            let full_line = if active_buffer.copy_line(line_index, &mut line_buf).is_ok() {
                line_buf
            } else {
                let mut non_existant_line = " ".repeat((viewport_size.0 - 1) as _);
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
        self.compute_selection_spans(&mut panel_spans, &active_buffer);

        // Wooowie done
        UiPanel {
            position: (0, 0),
            size: viewport_size,
            content: panel_content,
            spans: panel_spans,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AnchorNextState {
    Unset,
    JustSet,
    Set,
}
