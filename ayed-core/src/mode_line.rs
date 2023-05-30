use crate::{
    core::EditorContextMut,
    input::Input,
    panel::Panel,
    panels::mode_line_panel::{ModeLineInfo, ModeLinePanel},
    ui_state::UiPanel,
};

pub struct ModeLine {
    panel: ModeLinePanel,
}

impl ModeLine {
    pub fn new() -> Self {
        Self {
            panel: ModeLinePanel::new(),
        }
    }

    pub fn has_focus(&self) -> bool {
        self.panel.has_focus()
    }

    pub fn set_has_focus(&mut self, has_focus: bool) {
        self.panel.set_has_focus(has_focus);
    }

    pub fn set_infos(&mut self, infos: Vec<ModeLineInfo>) {
        self.panel.set_infos(infos);
    }

    pub fn input(&mut self, input: Input, ctx: &mut EditorContextMut) -> Option<String> {
        let commands = self.panel.convert_input_to_command(input, ctx);
        for command in commands {
            let maybe_line = self.panel.send_command(command, ctx);
            if maybe_line.is_some() {
                return maybe_line;
            }
        }
        None
    }

    pub fn ui_panel(&mut self, ctx: &EditorContextMut) -> UiPanel {
        self.panel.panel(ctx)
    }
}
