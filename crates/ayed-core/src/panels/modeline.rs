use crate::{
    position::Position,
    ui::{
        Color, Rect, Style, theme,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::line_builder::LineBuilder,
};

use super::{Editor, FocusedPanel, RenderPanelContext};

pub const FG_COLOR: Color = theme::colors::MODELINE_TEXT;
pub const BG_COLOR: Color = theme::colors::ACCENT;

#[derive(Default)]
pub struct Modeline {
    // FIXME modeline should probably keep the same view and buffer always,
    // instead of taking it from FocusedPanel.
    rect: Rect,
}

impl Modeline {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &RenderPanelContext) -> UiPanel {
        let size = self.rect.size();

        if let FocusedPanel::Modeline(view_handle) = ctx.state.focused_panel {
            let mut editor = Editor::with_view(view_handle);
            editor.set_rect(Rect::from_positions(
                self.rect.top_left(),
                self.rect.bottom_right(),
            ));

            let mut editor_panel = editor.render(ctx);

            for line in &mut editor_panel.content {
                line.insert(0, '›');
            }

            for region in &mut editor_panel.spans {
                region.from = region.from.offset((1, 0));
                region.to = region.to.offset((1, 0));
            }

            // Prompt color
            editor_panel.spans.push(StyledRegion {
                from: Position::ZERO,
                to: Position::ZERO,
                style: Style {
                    foreground_color: None,
                    background_color: Some(theme::colors::ACCENT_BRIGHT),
                    ..Default::default()
                },
                priority: 1,
            });

            // Bg color
            editor_panel.spans.push(StyledRegion {
                from: Position::ZERO,
                to: Position::ZERO.offset((self.rect().width as _, 0)),
                style: Style {
                    foreground_color: None,
                    background_color: Some(theme::colors::ACCENT),
                    ..Default::default()
                },
                priority: 0,
            });

            editor_panel
        } else {
            let mut line_builder = LineBuilder::new_with_length(size.column as _);

            let mut style = Style {
                foreground_color: Some(FG_COLOR),
                background_color: Some(BG_COLOR),
                ..Default::default()
            };

            if let Some(content_override) = &ctx.state.modeline.content_override {
                line_builder = line_builder.add_left_aligned(&content_override.text, ());
                style = content_override.style;
            } else {
                for info in ctx.state.modeline.infos.iter() {
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
                size,
                content: vec![content],
                spans: vec![StyledRegion {
                    from: Position::ZERO,
                    to: Position::ZERO
                        .with_column(size.column.saturating_sub(1).try_into().unwrap()),
                    style,
                    ..Default::default()
                }],
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ModelineState {
    pub infos: Vec<ModelineInfo>,
    pub content_override: Option<ContentOverride>,
    pub history: Vec<String>,
    pub history_selected_item: usize,
}

impl ModelineState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_message(&mut self, text: String) {
        self.content_override = Some(ContentOverride {
            text,
            style: Style {
                ..Default::default()
            },
        });
    }

    pub fn set_error(&mut self, text: String) {
        self.content_override = Some(ContentOverride {
            text,
            style: Style {
                foreground_color: Some(theme::colors::MODELINE_TEXT),
                background_color: Some(theme::colors::ERROR_DARK),
                ..Default::default()
            },
        });
    }

    pub fn clear_content_override(&mut self) {
        self.content_override = None;
    }

    pub fn iter(&self) -> impl Iterator<Item = &ModelineInfo> + '_ {
        self.infos.iter()
    }
}

#[derive(Debug, Clone)]
pub struct ModelineInfo {
    pub text: String,
    pub style: Style,
    pub align: Align,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct ContentOverride {
    pub text: String,
    pub style: Style,
}
