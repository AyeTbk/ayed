use crate::{
    panels::{LayoutContext, LayoutInfo, LayoutPlace, Panel},
    position::Position,
    selection::Selections,
    slotmap::Handle,
    state::{Align, TextBuffer, View},
    ui::{
        Rect, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::line_builder::LineBuilder,
};

use super::PanelContext;

#[derive(Default)]
pub struct StatusBar {
    rect: Rect,
    view_handle: Option<Handle<View>>,
}

impl Panel for StatusBar {
    fn layout_info(&self, _ctx: &LayoutContext) -> LayoutInfo {
        LayoutInfo {
            place: LayoutPlace::South,
            height: Some(1),
            ..Default::default()
        }
    }

    fn rect(&self) -> Rect {
        self.rect()
    }

    fn set_rect(&mut self, rect: Rect) {
        self.set_rect(rect)
    }

    fn on_focus(&mut self, ctx: &mut PanelContext) {
        let buffer = ctx
            .resources
            .buffers
            .insert(TextBuffer::new_internal("status-bar"));
        let view = ctx.resources.views.insert(View {
            top_left: Position::ZERO,
            buffer,
        });
        ctx.resources
            .buffers
            .get_mut(buffer)
            .set_view_selections(view, Selections::new());
        self.view_handle = Some(view);
    }

    fn on_unfocus(&mut self, ctx: &mut PanelContext) {
        let Some(view_handle) = self.view_handle else { return };
        let buffer_handle = ctx.resources.views.get(view_handle).buffer;
        ctx.resources.views.remove(view_handle);
        ctx.resources.buffers.remove(buffer_handle);
        self.view_handle = None;
    }

    fn render(&self, ctx: &PanelContext) -> Vec<UiPanel> {
        vec![self.render(ctx)]
    }

    fn name(&self) -> Option<&str> {
        Some("status-bar")
    }
}

impl StatusBar {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &PanelContext) -> UiPanel {
        // TODO clean up this mess

        let size = self.rect.size();

        let mut spans = Vec::new();

        let mut top_line_builder = LineBuilder::new().with_length(size.column as _);

        let mut top_style = Style {
            foreground_color: ctx.state.config.get_theme_color("modeline-text"),
            background_color: ctx.state.config.get_theme_color("accent"),
            ..Default::default()
        };

        if let Some(content_override) = &ctx.state.modeline.content_override {
            if let Some(style) = content_override.top_style {
                top_style = style;
            }
        }

        for info in ctx.state.modeline.infos.iter() {
            // TODO styles for the infos
            match info.align {
                Align::Right => {
                    top_line_builder =
                        top_line_builder.add_right_aligned(&info.text, Some(info.style));
                    top_line_builder = top_line_builder.add_right_aligned("  ", None);
                }
                Align::Left => {
                    top_line_builder =
                        top_line_builder.add_left_aligned(&info.text, Some(info.style));
                    top_line_builder = top_line_builder.add_left_aligned("  ", None);
                }
            }
        }

        let (top_line_content, styles) = top_line_builder.build();

        // Styles spans
        for positioned_style in styles {
            let Some(style) = positioned_style.data else { continue };
            spans.push(StyledRegion {
                from: Position::ZERO.offset((positioned_style.chars.0 as i32, 0)),
                to: Position::ZERO.offset((positioned_style.chars.1 as i32 - 1, 0)),
                style,
                ..Default::default()
            })
        }

        // Top Bg color
        spans.push(StyledRegion {
            from: Position::ZERO,
            to: Position::ZERO.with_column(size.column.saturating_sub(1).try_into().unwrap()),
            style: top_style,
            ..Default::default()
        });

        UiPanel {
            position: self.rect.top_left(),
            size,
            content: vec![top_line_content],
            spans,
        }
    }
}
