use crate::{
    buffer::TextBuffer,
    command::Command,
    selection::{DeletedEditInfo, EditInfo, Position, Selection, Selections},
    state::State,
    ui_state::{Color, Span, Style, UiPanel},
    utils::Rect,
};

pub struct TextBufferEdit {
    rect: Rect,
    selections: Selections,
    view_top_left_position: Position,
    anchor_down: bool,
    anchor_next: AnchorNextState,
    pub use_alt_cursor_style: bool,
}

impl TextBufferEdit {
    pub fn new() -> Self {
        Self {
            rect: Rect::new(0, 0, 25, 25),
            selections: Selections::new(),
            view_top_left_position: Position::ZERO,
            anchor_down: false,
            anchor_next: AnchorNextState::Unset,
            use_alt_cursor_style: false,
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn view_top_left_position(&self) -> Position {
        self.view_top_left_position
    }

    pub fn execute_command(
        &mut self,
        command: Command,
        buffer: &mut TextBuffer,
        state: &mut State,
    ) {
        use Command::*;
        match command {
            AnchorNext => self.lower_anchor(true),
            AnchorDown => self.lower_anchor(false),
            AnchorUp => self.raise_anchor(),

            Insert(ch) => self.insert_char_for_each_selection(ch, buffer),
            DeleteSelection => self.delete_selection_for_each_selection(buffer),
            DeleteCursor => self.delete_cursor_for_each_selection(buffer),
            DeleteBeforeCursor => self.delete_before_cursor_for_each_selection(buffer),

            // Wow
            MoveCursorUp => self.move_cursor_vertically(-1, buffer, self.anchored()),
            MoveCursorDown => self.move_cursor_vertically(1, buffer, self.anchored()),
            MoveCursorLeft => self.move_cursor_horizontally(-1, buffer, self.anchored()),
            MoveCursorRight => self.move_cursor_horizontally(1, buffer, self.anchored()),
            //
            MoveCursorTo(_, _) => todo!(),
            SetSelection { cursor, anchor } => {
                let selection = Selection::new().with_cursor(cursor).with_anchor(anchor);
                self.selections = Selections::new_with(selection, &[]);
            }
            //
            MoveCursorToLineStart => self.move_cursor_to_line_start(buffer, self.anchored()),
            MoveCursorToLineEnd => self.move_cursor_to_line_end(buffer, self.anchored()),
            //
            ShrinkSelectionToCursor => self.map_selections(|sel| sel.shrunk_to_cursor()),
            FlipSelection => self.map_selections(|sel| sel.flipped()),
            FlipSelectionForward => self.map_selections(|sel| sel.flipped_forward()),
            FlipSelectionBackward => self.map_selections(|sel| sel.flipped_forward().flipped()),
            //
            DuplicateSelectionAbove => self.duplicate_selection_above_or_below(buffer, true),
            DuplicateSelectionBelow => self.duplicate_selection_above_or_below(buffer, false),

            cmd => unimplemented!("{:?}", cmd),
        }

        self.anchor_check();

        self.normalize_selections();

        self.adjust_viewport_to_primary_selection(state);
    }

    pub fn render(&mut self, buffer: &TextBuffer, state: &State) -> UiPanel {
        let viewport_size = self.rect.size();

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
            let full_line = if buffer.copy_line(line_index, &mut line_buf).is_ok() {
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
        self.compute_selection_spans(&mut panel_spans, buffer);

        UiPanel {
            position: (0, 0),
            size: viewport_size,
            content: panel_content,
            spans: panel_spans,
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
        let selection_color = if self.use_alt_cursor_style {
            Some(Color::rgb(150, 32, 96))
        } else {
            Some(Color::rgb(18, 72, 139))
        };

        for (selection, selection_split_by_line) in self.selections_split_by_lines(buffer) {
            let cursor_color = if self.use_alt_cursor_style {
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

    fn adjust_viewport_to_primary_selection(&mut self, _state: &State) {
        let mut new_viewport_top_left_position = self.view_top_left_position;
        // Horizontal
        let vp_start_x = self.view_top_left_position.column_index;
        let vp_after_end_x = vp_start_x + self.rect.width;
        let selection_x = self.selections.primary().cursor().column_index;

        if selection_x < vp_start_x {
            new_viewport_top_left_position.column_index = selection_x;
        } else if selection_x >= vp_after_end_x {
            new_viewport_top_left_position.column_index = selection_x - self.rect.width + 1;
        }

        // Vertical
        let vp_start_y = self.view_top_left_position.line_index;
        let vp_after_end_y = vp_start_y + self.rect.height;
        let selection_y = self.selections.primary().cursor().line_index;

        if selection_y < vp_start_y {
            new_viewport_top_left_position.line_index = selection_y;
        } else if selection_y >= vp_after_end_y {
            new_viewport_top_left_position.line_index = selection_y - self.rect.height + 1;
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
}

#[derive(Debug, Clone, Copy)]
enum AnchorNextState {
    Unset,
    JustSet,
    Set,
}