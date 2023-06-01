use crate::{
    controls::LineEdit,
    input::Input,
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
}

impl ModeLine {
    pub fn new() -> Self {
        Self {
            has_focus: Default::default(),
            line_edit: LineEdit::new(),
            rect: Rect::new(0, 0, 25, 1),
        }
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

    pub fn input(&mut self, input: Input, state: &mut State) -> Option<String> {
        self.line_edit.set_rect(self.rect);
        self.line_edit.input(input, state)
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let mut line_builder = LineBuilder::new_with_length(self.rect.width as _);

        for info in state.mode_line_infos.iter() {
            // TODO styles for the infos
            line_builder = line_builder.add_right_aligned(&info.text, ());
        }

        if self.has_focus() {
            // TODO unify this with the rest maybe idk figure it out
            self.line_edit.set_rect(self.rect);
            return self.line_edit.render(state);
        }

        let (content, _) = line_builder.build();

        UiPanel {
            position: self.rect.top_left(),
            size: self.rect.size(),
            content: vec![content],
            spans: vec![Span {
                from: Position::ZERO,
                to: Position::ZERO.with_moved_indices(0, self.rect.width as _),
                style: Style {
                    foreground_color: Some(Color::rgb(200, 200, 0)),
                    background_color: None,
                    invert: false,
                },
                importance: 1,
            }],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModeLineInfo {
    pub text: String,
    pub style: Style,
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
