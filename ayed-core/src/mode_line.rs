use crate::{
    command::Command,
    controls::LineEdit,
    line_builder::LineBuilder,
    selection::Position,
    state::State,
    ui_state::{Color, Span, Style, UiPanel},
    utils::Rect,
};

pub struct ModeLine {
    has_focus: bool,
    line_edit: LineEdit,
    rect: Rect,
    content_override: Option<ContentOverride>,
}

impl ModeLine {
    pub fn new() -> Self {
        Self {
            has_focus: Default::default(),
            line_edit: LineEdit::new(),
            rect: Rect::new(0, 0, 25, 1),
            content_override: None,
        }
    }

    pub fn set_content_override(&mut self, content_override: Option<ContentOverride>) {
        self.content_override = content_override;
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn has_focus(&self) -> bool {
        self.has_focus
    }

    pub fn set_has_focus(&mut self, has_focus: bool) {
        self.has_focus = has_focus;
    }

    pub fn execute_command(&mut self, command: Command, state: &mut State) -> Option<String> {
        self.line_edit.set_rect(self.rect);
        self.line_edit.execute_command(command, state)
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        if self.has_focus() {
            // TODO unify this with the rest maybe idk figure it out
            self.line_edit.set_rect(self.rect);
            return self.line_edit.render(state);
        }

        let mut line_builder = LineBuilder::new_with_length(self.rect.width as _);
        let mut style = Style {
            foreground_color: Some(Color::rgb(200, 200, 0)),
            background_color: Some(Color::rgb(40, 30, 50)),
            invert: false,
        };

        if let Some(content_override) = &self.content_override {
            line_builder = line_builder.add_left_aligned(&content_override.text, ());
            style = content_override.style;
        } else {
            for info in state.mode_line_infos.iter() {
                // TODO styles for the infos
                match info.align {
                    Align::Right => {
                        line_builder = line_builder.add_right_aligned(&info.text, ());
                        line_builder = line_builder.add_right_aligned("  ", ());
                    }
                    Align::Left => {
                        line_builder = line_builder.add_left_aligned(&info.text, ());
                        line_builder = line_builder.add_left_aligned("  ", ());
                    }
                }
            }
        }

        let (content, _) = line_builder.build();

        UiPanel {
            position: self.rect.top_left(),
            size: self.rect.size(),
            content: vec![content],
            spans: vec![Span {
                from: Position::ZERO,
                to: Position::ZERO.with_moved_indices(0, self.rect.width as _),
                style,
                importance: 1,
            }],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModeLineInfo {
    pub text: String,
    pub style: Style,
    pub align: Align,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

#[derive(Debug, Default, Clone)]
pub struct ModeLineInfos {
    // infos: HashMap<String, ModeLineInfo>,
    pub(crate) infos: Vec<ModeLineInfo>,
}

impl ModeLineInfos {
    pub fn new() -> Self {
        Self {
            infos: Default::default(),
        }
    }

    // pub fn set(&mut self, key: impl Into<String>, info: ModeLineInfo) {
    //     self.infos.insert(key.into(), info);
    // }

    // pub fn unset(&mut self, key: &str) {
    //     self.infos.remove(key);
    // }

    // pub fn iter(&self) -> impl Iterator<Item = &ModeLineInfo> + '_ {
    //     self.infos.values()
    // }

    pub fn iter(&self) -> impl Iterator<Item = &ModeLineInfo> + '_ {
        self.infos.iter()
    }
}

pub struct ContentOverride {
    pub text: String,
    pub style: Style,
}
