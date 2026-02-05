use crate::{
    panels::RenderPanelContext,
    position::{Column, Position, Row},
    ui::{
        Rect, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::render_utils::{BORDER_ALL, decorated_rectangle},
};

#[derive(Default)]
pub struct HoverInfo {
    rect: Rect,
}

impl HoverInfo {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &RenderPanelContext) -> Vec<UiPanel> {
        let Some(text) = &ctx.state.hover_info else {
            return Vec::new();
        };

        let boxfg = ctx.state.config.get_theme_color("box-fg");
        let boxbg = ctx.state.config.get_theme_color("box-bg");
        let text_color = ctx.state.config.get_theme_color("editor-fg");

        let default_style = Style {
            background_color: boxbg,
            foreground_color: boxfg,
            ..Default::default()
        };

        let back_panel = decorated_rectangle(
            self.rect.top_left(),
            self.rect.size(),
            default_style,
            BORDER_ALL,
        );

        let text_rect = self.rect.grown(-1, -1, -2, -2);
        let text_panel = UiPanel {
            content: text.split_terminator('\n').map(str::to_string).collect(),
            position: text_rect.top_left(),
            size: text_rect.size(),
            spans: vec![StyledRegion {
                from: Position::ZERO,
                to: Position::new(Column::MAX, Row::MAX),
                style: Style {
                    foreground_color: text_color,
                    background_color: boxbg,
                    ..Default::default()
                },
                ..Default::default()
            }],
        };

        vec![back_panel, text_panel]
    }
}
