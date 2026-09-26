use ayed_core::core::Core;

mod tui;

fn main() {
    let mut core = Core::with_builtins();

    let mut any_path_specified = false;
    let mut args = std::env::args().skip(1);
    loop {
        let Some(arg) = args.next() else { break };
        if let Some(opt) = arg.strip_prefix("--") {
            match opt {
                "workspace" => {
                    let dir_path = args.next().unwrap();
                    let dir = std::path::absolute(dir_path).unwrap();
                    assert!(dir.is_dir());
                    core.state.working_directory = dir;
                }
                _ => {
                    eprintln!("error: unknown option '{opt}'");
                    std::process::exit(-1);
                }
            }
        } else {
            core.queue_command(format!("edit {arg}"));
            any_path_specified = true;
        }
    }
    if !any_path_specified {
        core.queue_command("edit --scratch".to_string());
    }
    core.tick();

    let mut tui = tui::Tui::new(core);

    tui.run();
}
