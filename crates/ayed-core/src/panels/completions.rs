use crate::{
    panels::{LayoutContext, LayoutInfo, LayoutPlace, Panel},
    position::Position,
    state::{CompletionItemKind, CompletionSource},
    ui::{
        Rect, Size, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::line_builder::LineBuilder,
};

use super::PanelContext;

#[derive(Default)]
pub struct Completions {
    rect: Rect,
}

impl Panel for Completions {
    fn layout_info(&self, _ctx: &LayoutContext) -> LayoutInfo {
        LayoutInfo {
            place: LayoutPlace::ManuallyManaged,
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

impl Completions {
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &PanelContext) -> Vec<UiPanel> {
        if ctx.state.completions.items.is_empty() {
            return vec![];
        }

        let placement = ctx.state.config.get_entry_value("completions", "placement");
        match placement {
            Ok("cursor") => self.render_at_cursor(ctx),
            Ok("prompt") => self.render_at_modeline(ctx).into_iter().collect(),
            Ok(_) => self.render_at_cursor(ctx),
            Err(_) => vec![],
        }
    }

    fn render_at_cursor(&self, ctx: &PanelContext) -> Vec<UiPanel> {
        // Need:
        // label, details, pad, icon(?) -> Use line builder
        // compute width from all content? -> Nope, fixed size, from quick heuristic?

        let displayed_item_count = i32::min(9, ctx.state.completions.items.len() as i32);
        let displayed_items_offset = i32::clamp(
            ctx.state.completions.selected_item - (displayed_item_count / 2 + 1),
            0,
            ctx.state.completions.items.len() as i32 - displayed_item_count,
        );

        let labels_width = ctx
            .state
            .completions
            .items
            .iter()
            .map(|i| i.label.len() + i.type_annotation.as_ref().map(|s| s.len()).unwrap_or(0) + 7)
            .max()
            .unwrap_or(0) as i32;
        let width = i32::clamp(labels_width, 12, 60);
        let height = displayed_item_count;
        let size = Size::new(width, height);

        let position_in_buffer = ctx.state.completions.original_symbol_start;
        let view_top_left = ctx.state.focused_panel_view_rect.unwrap().top_left();
        let target_position = position_in_buffer.local_to_pos(view_top_left)
            + ctx.state.editor_rect.top_left()
            + Position::new(ctx.state.editor_line_numbers_width, 0);
        let mut position = target_position;

        // Place on the line below the cursor
        position = position.offset((-1, 1));

        // Don't let the panel go past the end of the viewport, rightward
        if position.column + width >= ctx.state.viewport_size.column as i32 {
            let corrected_column = ctx.state.viewport_size.column as i32 - width;
            position = position.with_column(corrected_column);
        }

        // Don't let the panel go past the end of the viewport, leftward
        if position.column < 0 {
            position = position.with_column(0);
        }

        // Don't let the panel go past the end of the viewport, downward
        if position.row + height >= ctx.state.viewport_size.row as i32 {
            let corrected_row = target_position.row - height;
            position = position.with_row(corrected_row);
        }

        let mut content = Vec::new();
        let mut spans = Vec::new();
        let items = ctx.state.completions.items.iter();
        let displayed_items = items
            .skip(displayed_items_offset.try_into().unwrap())
            .take(displayed_item_count.try_into().unwrap());

        let color_accent_bright = ctx.state.config.get_theme_color("accent-bright");
        let color_accent_mild = ctx.state.config.get_theme_color("accent-mild");
        let color_editor_fg = ctx.state.config.get_theme_color("editor-fg");
        let color_editor_fg_mild = ctx.state.config.get_theme_color("editor-fg-mild");
        let sntx = ctx.state.config.get_syntax_sytle();

        for (i, item) in displayed_items.enumerate() {
            let i_idx = i as i32 + displayed_items_offset;

            // FIXME maybe this mapping should be where CompletionItemKind is defined
            let syntax_rule = match item.kind {
                CompletionItemKind::Plaintext => "<anything else>",
                CompletionItemKind::Keyword => "keyword", // FIXME syntax currently distinguishes between keywords and statement-keywords, but not rust analyzer. How fix?
                CompletionItemKind::Function => "function", // FIXME rust-analyzer reports macros as functions...
                CompletionItemKind::Variable => "variable",
                CompletionItemKind::Type => "type",
                CompletionItemKind::Interface => "interface",
                CompletionItemKind::Member => "member",
                CompletionItemKind::Module => "namespace", // TODO Should these be uniformized?
            };
            let fg_color = sntx.get(syntax_rule).and_then(|s| s.color);

            let mut builder = LineBuilder::new().with_length(width as usize);
            builder = builder
                .add_left_aligned(" ", None) // place for scrollbar
                .add_left_aligned(&item.label, fg_color);
            if let Some(type_annotation) = &item.type_annotation {
                builder = builder
                    .add_left_aligned(": ", color_editor_fg_mild)
                    .add_left_aligned(type_annotation, color_editor_fg_mild);
            }
            if item.source == CompletionSource::Buffer {
                builder = builder.add_right_aligned(" 🗎 ", color_editor_fg_mild);
            }
            let (line, colors) = builder.build();
            let bg_color = if ctx.state.completions.selected_item == i_idx + 1 {
                color_accent_bright
            } else {
                color_accent_mild
            };

            spans.push(StyledRegion {
                from: Position::new(0, i as i32),
                to: Position::new(width - 1, i as i32),
                style: Style {
                    background_color: bg_color,
                    ..Default::default()
                },
                priority: 2,
            });
            for color in colors {
                spans.push(StyledRegion {
                    from: Position::new(color.chars.0 as i32, i as i32),
                    to: Position::new(color.chars.1 as i32 - 1, i as i32),
                    style: Style {
                        foreground_color: color.data.or(color_editor_fg),
                        background_color: bg_color,
                        ..Default::default()
                    },
                    priority: 4,
                });
            }

            content.push(line);
        }

        let main_panel = UiPanel {
            position,
            size,
            content,
            spans,
        };

        let mut panels = vec![main_panel];

        // Scroll bar
        if height != ctx.state.completions.items.len() as i32 {
            let window_size = height * height / ctx.state.completions.items.len() as i32;
            let window_start =
                (displayed_items_offset * height) / ctx.state.completions.items.len() as i32;
            let window_end = window_start + window_size;
            let mut scrollbar_content: Vec<String> = Vec::new();
            for i in 0..height {
                if window_start <= i && i <= window_end {
                    scrollbar_content.push(String::from("▌"));
                } else {
                    scrollbar_content.push(String::from(" "));
                }
            }
            let scrollbar_panel = UiPanel {
                position,
                size: Size::new(1, height),
                content: scrollbar_content,
                spans: vec![StyledRegion {
                    from: Position::ZERO,
                    to: Position::ZERO.offset((0, height)),
                    style: Style {
                        foreground_color: color_accent_bright,
                        background_color: color_accent_mild,
                        ..Default::default()
                    },
                    ..Default::default()
                }],
            };

            panels.push(scrollbar_panel);
        }

        panels
    }

    fn render_at_modeline(&self, ctx: &PanelContext) -> Option<UiPanel> {
        // NOTE This hasnt been kept up to date, consider it deletable trash
        let width = ctx.state.modeline_rect.width;
        let height = ctx.state.completions.items.len() as i32;
        let size = Size::new(width, height);

        let position = ctx.state.modeline_rect.top_left().offset((0, -height));

        let mut content = Vec::new();
        let mut spans = Vec::new();
        for (i, item) in ctx.state.completions.items.iter().enumerate() {
            let mut s = item.label.clone();
            let pad = " ".repeat(width as usize - s.len());
            s.push_str(&pad);
            content.push(s);

            let color = if ctx.state.completions.selected_item == (i as i32 + 1) {
                ctx.state.config.get_theme_color("accent-bright")
            } else {
                ctx.state.config.get_theme_color("accent-mild")
            };
            spans.push(StyledRegion {
                from: Position::new(0, i as i32),
                to: Position::new(width as i32, i as i32),
                style: Style {
                    foreground_color: None,
                    background_color: color,
                    ..Default::default()
                },
                priority: 0,
            });
        }

        Some(UiPanel {
            position,
            size,
            content,
            spans,
        })
    }
}
