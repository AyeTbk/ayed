use crate::{
    panels::{LayoutContext, LayoutInfo, LayoutPlace, Panel},
    position::Position,
    selection::Selections,
    slotmap::Handle,
    state::{TextBuffer, View},
    ui::{
        Rect, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::{char_count, line_builder::LineBuilder},
};

use super::{Editor, PanelContext};

#[derive(Default)]
pub struct Prompt {
    rect: Rect,
    view_handle: Option<Handle<View>>,
}

impl Panel for Prompt {
    fn layout_info(&self, _ctx: &LayoutContext) -> LayoutInfo {
        LayoutInfo {
            place: LayoutPlace::South,
            height: Some(1),
            ..Default::default()
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn set_rect(&mut self, rect: Rect) {
        self.set_rect(rect)
    }

    fn on_focus(&mut self, ctx: &mut PanelContext) {
        let buffer = ctx.resources.buffers.insert(TextBuffer::new_empty());
        let view = ctx.resources.views.insert(View {
            top_left: Position::ZERO,
            buffer,
        });
        ctx.resources
            .buffers
            .get_mut(buffer)
            .add_view_selections(view, Selections::new());
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
        self.render(ctx)
    }

    fn name(&self) -> Option<&str> {
        Some("prompt")
    }

    fn view(&self) -> Option<Handle<View>> {
        self.view_handle
    }
}

impl Prompt {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &PanelContext) -> Vec<UiPanel> {
        let size = self.rect.size();
        let mut panels = Vec::new();

        let mut bg_style = Style {
            background_color: ctx.state.config.get_theme_color("editor-bg"),
            ..Default::default()
        };
        let bg_content;
        if let Some(content_override) = &ctx.state.modeline.content_override {
            if let Some(style) = content_override.bottom_style {
                bg_style = style;
            }
            bg_content = LineBuilder::new()
                .with_length(size.column as _)
                .add_left_aligned(&content_override.text, ())
                .build()
                .0;
        } else {
            bg_content = " ".repeat(size.column as _);
        }

        // Background uipanel
        panels.push(UiPanel {
            position: self.rect.top_left(),
            size,
            content: vec![bg_content],
            spans: vec![StyledRegion {
                from: Position::ZERO,
                to: Position::ZERO.offset((size.column - 1, 0)),
                style: bg_style,
                ..Default::default()
            }],
        });

        if let Some(view_handle) = self.view_handle {
            // Prompt uipanel
            let prompt_mode = ctx
                .state
                .config
                .state_value("prompt-mode")
                .unwrap_or_default();
            let prompt_text = format!("{}{}", prompt_mode, '›');
            let prompt_text_len = char_count(&prompt_text);
            panels.push(UiPanel {
                position: self.rect.top_left(),
                size,
                content: vec![prompt_text],
                spans: vec![StyledRegion {
                    from: Position::ZERO,
                    to: Position::ZERO.offset((prompt_text_len.saturating_sub(1) as _, 0)),
                    style: Style {
                        foreground_color: ctx.state.config.get_theme_color("modeline-text"),
                        background_color: ctx.state.config.get_theme_color("accent"),
                        ..Default::default()
                    },
                    ..Default::default()
                }],
            });

            // Editor uipanel
            let mut editor = Editor::with_view(view_handle);
            let rect = self.rect.grown(0, 0, -(prompt_text_len as i32), 0);
            editor.set_rect(rect);
            let mut editor_panel = editor.render(ctx).pop().unwrap();

            // Poor man's try block
            (|| -> Option<()> {
                let content = editor_panel.content.first_mut()?;
                let prompt_empty = content.trim().is_empty();
                if !prompt_empty {
                    return None;
                }
                let history = ctx.state.modeline.histories.get(prompt_mode)?;
                let entry = history.entries.last()?;

                *content = LineBuilder::new()
                    .with_length(char_count(content))
                    .add_left_aligned(entry, ())
                    .build()
                    .0;

                editor_panel.spans.push(StyledRegion {
                    from: Position::ZERO,
                    to: Position::ZERO.offset((char_count(entry) as _, 0)),
                    style: Style {
                        foreground_color: ctx.state.config.get_theme_color("modeline-text-dim"),
                        ..Default::default()
                    },
                    priority: 127,
                    ..Default::default()
                });

                Some(())
            })();

            panels.push(editor_panel);
        }

        panels
    }
}
