use std::sync::LazyLock;

use regex::Regex;

use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
    position::{Column, Position},
    selection::Selection,
    state::TextBuffer,
    utils::string_utils::{byte_index_to_char_index, char_index_to_byte_index},
};

static RE_SYMBOL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+|[^\s\w]+").unwrap());

// Should be per editor, cleared on changing which buffer is edited
#[derive(Default)]
pub struct CompletionsState {
    /// The position in the buffer where the cursor should be for the
    /// suggestion box to show up. Used to show hide the box when appropriate.
    pub prompt_suggestion_cursor_position: Option<Position>,
    pub items: Vec<String>,
    /// Selected item index, 1 based, where 0 means none.
    pub selected_item: i32,
    /// The original symbols for all the active view cursors.
    pub original_symbols: Vec<String>,
    /// The start position of the primary cursor's original symbol.
    pub original_symbol_start: Position,
}

pub fn register_completions_commands(cr: &mut CommandRegistry) {
    // completions-select
    // completions-gather (from current buffer)

    // Think of how from-buffer, LSP and modeline will fit into this.
    // When/how does completion suggestion box trigger?:
    //  - When there are suggestions
    // When/how do suggestions get added/cleared?:
    //  - Whenever the primary cursor is moved, if it falls within or
    //    at the end of a symbol, suggestions should be gathered from
    //    all sources.
    //      - Problem?: LSP completions request will take some time to
    //        arrive. In the case the user is typing, many requests will
    //        be fired, but only the last request matters. The response
    //        to the others should be ignored.
    //  - Problem: with multi cursor completions and from-buffer suggestions,
    //    when selecting a suggestion, the first choice can get hogged by the
    //    latest completion if done within a symbol.
    //      - Solution?: ignore for now, as known issue.
    // How are the many sources of suggestions reconciled?:
    //  - Everything gets unceremoniously slapped in `items`.
    // LSP needs to be kept up to date with the changes do to
    // the file. (document synchronization).

    cr.register("completions-select", |opt, ctx| {
        if ctx.state.completions.items.is_empty() {
            return Ok(());
        }

        let opts = Options::new().flag("next").flag("previous").parse(opt)?;
        let next = opts.contains("next");
        let previous = opts.contains("previous");

        let cycling_from_original = ctx.state.completions.selected_item == 0;
        ctx.state.completions.selected_item += next as i32 - (previous as i32);
        let modulo = ctx.state.completions.items.len() as i32 + 1;
        ctx.state.completions.selected_item =
            ctx.state.completions.selected_item.rem_euclid(modulo);
        let selected_item_idx = i32::max(ctx.state.completions.selected_item - 1, 0) as usize;
        let cycling_to_original = ctx.state.completions.selected_item == 0;

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        // Select the symbols under cursor in order to delete and replace it later.
        for sel_idx in 0..sel_count {
            let selections = buffer.view_selections(view_handle).unwrap();
            let sel = selections.get(sel_idx).unwrap();
            let new_sel = selection_from_symbol_prefix_under_cursor(buffer, sel.cursor);
            let selections = buffer.view_selections_mut(view_handle).unwrap();
            let sel = selections.get_mut(sel_idx).unwrap();
            *sel = new_sel;

            if cycling_from_original {
                // Gather original symbols
                ctx.state.completions.original_symbols.clear();

                let selections = buffer.view_selections(view_handle).unwrap();
                let sel = selections.get(sel_idx).unwrap();
                let original_sel = sel.with_end(sel.end().offset((-1, 0)));
                let original = buffer.selection_text(&original_sel);
                ctx.state
                    .completions
                    .original_symbols
                    .push(original.unwrap());
            }
        }

        // Delete symbols under cursors and replace with appropriate suggestion
        for sel_idx in 0..sel_count {
            let selections = buffer.view_selections(view_handle).unwrap();
            let sel = selections.get(sel_idx).unwrap();

            let delete_sel = sel.with_end(sel.end().offset((-1, 0)));
            buffer.delete_selection(&delete_sel)?;

            let text_to_insert;
            if cycling_to_original {
                text_to_insert = ctx.state.completions.original_symbols[sel_idx].as_str();
            } else {
                text_to_insert = ctx.state.completions.items[selected_item_idx].as_str();
            }
            let mut new_sel = buffer.insert_str_at(sel.start(), text_to_insert)?;
            new_sel = new_sel.with_end(new_sel.end().offset((1, 0)));

            ctx.state.completions.prompt_suggestion_cursor_position = Some(new_sel.end());

            let selections = buffer.view_selections_mut(view_handle).unwrap();
            *selections.get_mut(sel_idx).unwrap() = new_sel.shrunk_to_cursor();
        }

        ctx.queue.emit("buffer-modified", "");
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("completions-clear", |_opt, ctx| {
        // TODO In order for this to work properly, it would need to keep track of what
        // this position is over the modifications that happen in the buffer (in
        // particular, this makes the suggbox misbehave with multicursors before the
        // primary cursor).
        // TODO fix the above using a buffer mark when that's a thing.
        ctx.state.completions.prompt_suggestion_cursor_position = None;

        ctx.state.completions.items.clear();
        ctx.state.completions.selected_item = 0;

        Ok(())
    });

    cr.register(
        "completions-gather",
        focused_buffer_command(|_opt, ctx| {
            let source = ctx.state.config.get_entry_value("completions", "source")?;
            if source != "active-buffer" {
                return Err("only 'active-buffer' is supported as suggestion source".to_string());
            }

            let cursor = ctx.selections.primary().cursor;

            let should_prompt =
                Some(cursor) == ctx.state.completions.prompt_suggestion_cursor_position;

            if should_prompt && ctx.state.completions.selected_item != 0 {
                // Don't interfere with completions when user is selecting one.
                return Ok(());
            }

            let line = ctx.buffer.line(cursor.row).unwrap();

            let cursor_byte_idx =
                char_index_to_byte_index(line, cursor.column as _).unwrap_or(line.len());
            let mut maybe_symbol_prefix = None;
            let mut maybe_symbol_start_end = None;
            for matsh in RE_SYMBOL.find_iter(line) {
                if matsh.start() < cursor_byte_idx && matsh.end() >= cursor_byte_idx {
                    maybe_symbol_prefix =
                        Some((matsh.as_str(), &line[matsh.start()..cursor_byte_idx]));
                    maybe_symbol_start_end = Some((matsh.start(), matsh.end()));
                }
            }

            ctx.state.completions.items.clear();
            ctx.state.completions.selected_item = 0;

            if let Some((start_index, end_index)) = maybe_symbol_start_end {
                let start_column = byte_index_to_char_index(line, start_index).unwrap();
                let symbol_start = Position::new(start_column as Column, cursor.row);
                ctx.state.completions.original_symbol_start = symbol_start;

                let end_column = byte_index_to_char_index(line, end_index).unwrap();
                let prompt_position = Position::new(end_column as Column + 1, cursor.row);
                ctx.state.completions.prompt_suggestion_cursor_position = Some(prompt_position);
            }

            let Some((symbol, prefix)) = maybe_symbol_prefix else { return Ok(()) };
            // TODO bail if prefix hasnt changed (add prefix to suggs state)

            for i in 0..ctx.buffer.line_count() {
                let line = ctx.buffer.line(i).unwrap();
                for matsh in RE_SYMBOL.find_iter(line) {
                    let matsh_str = matsh.as_str();
                    if matsh_str.starts_with(prefix) && matsh_str != symbol {
                        let item = matsh_str.to_string();
                        if !ctx.state.completions.items.contains(&item) {
                            ctx.state.completions.items.push(item);
                        }
                    }
                }
            }

            Ok(())
        }),
    );
}

fn selection_from_symbol_prefix_under_cursor(buffer: &TextBuffer, cursor: Position) -> Selection {
    let row = cursor.row;
    let line = buffer.line(row).unwrap();
    let cursor_byte_idx = char_index_to_byte_index(line, cursor.column as _).unwrap();
    let mut maybe_selection = None;
    for matsh in RE_SYMBOL.find_iter(line) {
        if matsh.start() < cursor_byte_idx && matsh.end() >= cursor_byte_idx {
            let start_column = byte_index_to_char_index(line, matsh.start()).unwrap() as Column;
            // let end_column = byte_index_to_char_index(line, matsh.end()).unwrap() as Column;
            let end_column = cursor_byte_idx as Column;
            maybe_selection = Some(
                Selection::new()
                    .with_anchor((start_column, row).into())
                    .with_cursor((end_column, row).into()),
            )
        }
    }
    maybe_selection.unwrap_or_else(|| Selection::new().with_cursor(cursor).shrunk_to_cursor())
}
