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
    pub const HEIGHT: u32 = 2;
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &RenderPanelContext) -> UiPanel {
        // TODO clean up this mess

        let size = self.rect.size();

        let mut spans = Vec::new();

        let mut bottom_editor = None;
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
            spans.push(StyledRegion {
                from: Position::ZERO.offset((0, 1)),
                to: Position::ZERO.offset((0, 1)),
                style: Style {
                    foreground_color: None,
                    background_color: Some(theme::colors::ACCENT_BRIGHT),
                    ..Default::default()
                },
                priority: 1,
            });

            for span in &mut editor_panel.spans {
                span.from = span.from.offset((0, 1));
                span.to = span.to.offset((0, 1));
            }
            spans.extend(std::mem::take(&mut editor_panel.spans));

            bottom_editor = Some(editor_panel);
        }

        let mut top_line_builder = LineBuilder::new_with_length(size.column as _);
        let mut bottom_line_builder = LineBuilder::new_with_length(size.column as _);

        let mut top_style = Style {
            foreground_color: Some(FG_COLOR),
            background_color: Some(BG_COLOR),
            ..Default::default()
        };
        let mut bottom_style = Style {
            foreground_color: None,
            background_color: Some(theme::colors::ACCENT_DARK),
            ..Default::default()
        };

        if let Some(content_override) = &ctx.state.modeline.content_override {
            bottom_line_builder = bottom_line_builder.add_left_aligned(&content_override.text, ());
            if let Some(style) = content_override.top_style {
                top_style = style;
            }
            if let Some(style) = content_override.bottom_style {
                bottom_style = style;
            }
        }

        for info in ctx.state.modeline.infos.iter() {
            // TODO styles for the infos
            match info.align {
                Align::Right => {
                    top_line_builder = top_line_builder.add_right_aligned(&info.text, ());
                    top_line_builder = top_line_builder.add_right_aligned("  ", ());
                }
                Align::Left => {
                    top_line_builder = top_line_builder.add_left_aligned(&info.text, ());
                    top_line_builder = top_line_builder.add_left_aligned("  ", ());
                }
            }
        }

        let (top_line_content, _) = top_line_builder.build();
        let (mut bottom_line_content, _) = bottom_line_builder.build();

        if let Some(mut editor_panel) = bottom_editor {
            bottom_line_content = editor_panel.content.remove(0);
        }

        // Top Bg color
        spans.push(StyledRegion {
            from: Position::ZERO,
            to: Position::ZERO.with_column(size.column.saturating_sub(1).try_into().unwrap()),
            style: top_style,
            ..Default::default()
        });
        // Bottom Bg color
        spans.push(StyledRegion {
            from: Position::ZERO.offset((0, 1)),
            to: Position::ZERO.offset((self.rect().width as _, 1)),
            style: bottom_style,
            priority: 0,
        });

        UiPanel {
            position: self.rect.top_left(),
            size,
            content: vec![top_line_content, bottom_line_content],
            spans,
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
            ..Default::default()
        });
    }

    pub fn set_error(&mut self, text: String) {
        self.content_override = Some(ContentOverride {
            text,
            top_style: Some(Style {
                foreground_color: Some(theme::colors::MODELINE_TEXT),
                background_color: Some(theme::colors::ERROR),
                ..Default::default()
            }),
            bottom_style: Some(Style {
                foreground_color: Some(theme::colors::MODELINE_TEXT),
                background_color: Some(theme::colors::ERROR_DARK),
                ..Default::default()
            }),
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

#[derive(Debug, Default, Clone)]
pub struct ContentOverride {
    pub text: String,
    pub top_style: Option<Style>,
    pub bottom_style: Option<Style>,
}
