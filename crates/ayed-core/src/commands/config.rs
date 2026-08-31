use crate::{
    command::CommandRegistry,
    config::Config,
    input::Input,
    selection::Selection,
    state::{DiagnosticKind, Highlight, regex_syntax_highlight},
    ui::{Color, Style, style::UnderlineKind, ui_state::StyledRegion},
};

pub fn register_config_commands(cr: &mut CommandRegistry) {
    cr.register(
        "map-input",
        "Maps user inputs to command execution as configured in keybinds mappings.",
        |opt, ctx| {
            // hackish support for combo modes
            // FIXME see if you cant get this to work in the config
            let is_combo = ctx
                .state
                .config
                .state_value("mode")
                .is_some_and(|m| m.starts_with("combo-"));
            if is_combo {
                ctx.queue.set_state("mode", "normal");
            }

            // FIXME This whole thing is a mess, and it also sorta functions
            // like how hooks are handled and some logic is duplicated (arg
            // substitution handling).

            let input_str = opt.raw();
            let input =
                Input::parse(input_str).map_err(|_| format!("invalid input: {input_str}"))?;

            if let Some(cmds) = ctx.state.config.get_keybind(input) {
                for cmd in cmds {
                    ctx.queue.push(cmd);
                }
            } else if let Some(cmds) = ctx.state.config.get_keybind_else() {
                if cmds.len() == 1 {
                    if let Some(ch) = input.char() {
                        let mut cmd = cmds.first().expect("len is 1").to_string();
                        if cmd.find(Config::ARG_MARKER).is_some() {
                            let mut buf = [0u8; char::MAX_LEN_UTF8];
                            cmd = cmd.replace(Config::ARG_MARKER, ch.encode_utf8(&mut buf));
                        } else {
                            cmd = format!("{cmd} {ch}");
                        }
                        ctx.queue.push(cmd);
                    }
                } else {
                    for cmd in cmds {
                        let mut cmd = cmd.clone();
                        if let Some(ch) = input.char() {
                            if cmd.find(Config::ARG_MARKER).is_some() {
                                let mut buf = [0u8; char::MAX_LEN_UTF8];
                                cmd = cmd.replace(Config::ARG_MARKER, ch.encode_utf8(&mut buf));
                            }
                        }
                        ctx.queue.push(cmd);
                    }
                }
            }

            Ok(())
        },
    );

    cr.register(
        "generate-highlights",
        "Generate syntax highlighting",
        |_opt, ctx| {
            let Some(buffer_handle) = ctx.state.active_editor_buffer(&ctx.resources) else {
                return Ok(());
            };

            let buffer = ctx.resources.buffers.get(buffer_handle);
            let syntax = ctx.state.config.get_syntax();
            let syntax_style = ctx.state.config.get_syntax_sytle();
            let mut highlights =
                regex_syntax_highlight(buffer, syntax, syntax_style, &ctx.state.config);

            // FIXME improve this whole highlights mess and separate syntax from diagnostics etc
            if let Some(path) = buffer.path() {
                for diag in ctx.state.diagnostics.for_file(path) {
                    let diag_sel = Selection::from_range(diag.range);
                    let squiggle_color = match diag.kind {
                        DiagnosticKind::Error => Color::ERROR,
                        DiagnosticKind::Warning => Color::WARNING,
                        _ => Color::INFO,
                    };
                    highlights.push(Highlight {
                        styled_region: StyledRegion {
                            from: diag_sel.start(),
                            to: diag_sel.end(),
                            style: Style {
                                underline_color: Some(squiggle_color),
                                underlined: Some(UnderlineKind::Squiggle),
                                ..Default::default()
                            },
                            priority: 233,
                            ..Default::default()
                        },
                    });
                }
            }

            ctx.state
                .per_buffer
                .entry(buffer_handle)
                .or_default()
                .highlights = highlights;

            Ok(())
        },
    );
}
