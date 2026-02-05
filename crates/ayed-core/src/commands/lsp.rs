// Could there be a way to design a kind of "plugin" system
// or at least something modular, that would allow extending
// the editor in a less intrusive way? I feel the LSP
// functionalities would be a good candidate for that.

use ayed_lsp_client::{File, LspClient, Position, Request, Response};

use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command},
    utils::path_ext::PathExt,
};

pub fn register_lsp_commands(cr: &mut CommandRegistry) {
    cr.register("lsp-start", |_opt, ctx| {
        let mut client = LspClient::new();
        client.initialize();
        ctx.state.lsp_client = Some(client);
        Ok(())
    });

    cr.register("lsp-stop", |_opt, ctx| {
        if let Some(client) = ctx.state.lsp_client.take() {
            client.shutdown();
        }
        dbg!("turned off");
        Ok(())
    });

    cr.register("lsp-poll", |_opt, ctx| {
        let Some(client) = &mut ctx.state.lsp_client else {
            return Ok(());
        };

        client.tick();

        for response in client.receive_responses() {
            match response {
                Response::HoverInfo { text } => {
                    ctx.state.hover_info = Some(text);
                }
                _ => {
                    dbg!(response);
                }
            }
        }

        Ok(())
    });

    cr.register(
        "lsp-hover",
        focused_buffer_command(|_opt, ctx| {
            let Some(client) = &mut ctx.state.lsp_client else {
                return Err("lsp client not started".into());
            };

            let Some(path) = ctx.buffer.path() else {
                return Err("save the file before you can hover".into());
            };

            let filepath = path.to_str_or_err()?.to_string();
            let cursor = ctx.selections.primary().cursor;

            client.queue_request(Request::Hover {
                file: File(filepath),
                position: Position(cursor.column, cursor.row),
            });

            Ok(())
        }),
    );
}
