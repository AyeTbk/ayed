use crate::{
    panels::{LayoutContext, LayoutInfo, LayoutPlace, Panel, PanelContext, Sides},
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

impl Panel for HoverInfo {
    fn layout_info(&self, ctx: &LayoutContext) -> LayoutInfo {
        let mut place = LayoutPlace::FloatBottom;
        if let Some(view_handle) = ctx.state.active_editor_view {
            let view = ctx.resources.views.get(view_handle);
            let buffer = ctx.resources.buffers.get(view.buffer);
            if let Some(sels) = buffer.view_selections(view_handle) {
                let cursor = sels.primary_selection.cursor;
                let diff = cursor.row - view.top_left.row;
                let threshold = ctx.full_viewport_size.row / 2; // NOTE should actually be editor size but we
                if diff > threshold {
                    place = LayoutPlace::FloatTop;
                }
            }
        }

        LayoutInfo {
            place,
            height: Some(8),
            padding: Some(Sides {
                left: 2,
                right: 2,
                bottom: 1,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn set_rect(&mut self, rect: Rect) {
        self.set_rect(rect)
    }

    fn render(&self, ctx: &PanelContext) -> Vec<UiPanel> {
        self.render(ctx)
    }
}

impl HoverInfo {
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &PanelContext) -> Vec<UiPanel> {
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
