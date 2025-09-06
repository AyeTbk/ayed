use super::ExecuteCommandContext;

pub fn alias(
    original_command: impl Into<String>,
) -> impl Fn(&str, ExecuteCommandContext) -> Result<(), String> {
    let cmd = original_command.into();
    move |opt, ctx| {
        ctx.queue.push(format!("{cmd} {opt}"));
        Ok(())
    }
}
