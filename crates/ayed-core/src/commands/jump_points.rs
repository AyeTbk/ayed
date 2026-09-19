use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
    position::Position,
    selection::Selection,
};

pub fn register_jump_points_commands(cr: &mut CommandRegistry) {
    cr.register(
        "jump-save",
        Options::new().doc("nodoc"),
        focused_buffer_command(|opt, ctx| {
            let reg = opt.remainder().chars().next().unwrap_or('z');
            ctx.state
                .jump_points
                .registers
                .insert(reg, (ctx.buffer.path_str().to_string(), ctx.selections));
            let msg_cmd = if reg == 'z' {
                format!("message saved selections")
            } else {
                format!("message saved selections to '{reg}'")
            };
            ctx.queue.push(msg_cmd);
            Ok(())
        }),
    );

    cr.register(
        "jump-restore",
        Options::new().doc("nodoc"),
        focused_buffer_command(|opt, ctx| {
            let reg = opt.remainder().chars().next().unwrap_or('z');
            let Some((path_str, jump_sels)) = ctx.state.jump_points.registers.get(&reg) else {
                let err_msg = if reg == 'z' {
                    format!("no saved selections")
                } else {
                    format!("no saved selections in '{reg}'")
                };
                return Err(err_msg);
            };
            let mut jump_sels = jump_sels.clone();
            for jsel in jump_sels.iter_mut() {
                *jsel = ctx.buffer.limit_selection_to_content(jsel);
            }

            ctx.queue.push(format!("edit {}", path_str));
            ctx.queue
                .push(format!("selections-set {}", jump_sels.to_string()));

            ctx.queue.push("message restored selections");
            Ok(())
        }),
    );

    cr.register(
        "jump-extend",
        Options::new().doc("nodoc"),
        focused_buffer_command(|opt, ctx| {
            let reg = opt.remainder().chars().next().unwrap_or('z');
            let Some((path_str, jump_sels)) = ctx.state.jump_points.registers.get(&reg) else {
                let err_msg = if reg == 'z' {
                    format!("no saved selections")
                } else {
                    format!("no saved selections in '{reg}'")
                };
                return Err(err_msg);
            };

            if path_str != ctx.buffer.path_str() {
                return Err(format!("cannot extend from buffer '{path_str}'"));
            }

            let mut jump_sels = jump_sels.clone();
            for jsel in jump_sels.iter_mut() {
                *jsel = ctx.buffer.limit_selection_to_content(jsel);
            }

            let mut selections = ctx.selections;
            let zipped_sels = selections.iter_mut().zip(jump_sels.iter_mut());
            for (bsel, jsel) in zipped_sels {
                let start = Position::min(bsel.start(), jsel.start());
                let end = Position::max(bsel.end(), jsel.end());
                *bsel = Selection::new().with_start_and_end(start, end);
            }

            ctx.buffer.set_view_selections(ctx.view_handle, selections);

            ctx.queue.emit("selections-modified", "");

            ctx.queue.push("message extended selections");
            Ok(())
        }),
    );
}
