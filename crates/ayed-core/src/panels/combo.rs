use crate::{
    panels::{LayoutContext, LayoutInfo, LayoutPlace, Panel, RenderPanelContext},
    position::Position,
    state::State,
    ui::{
        Rect, Size, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::{
        render_utils::{BORDER_ALL, decorated_rectangle},
        string_utils::grid_string_builder::{Cell, GridStringBuilder},
    },
};

#[derive(Default)]
pub struct Combo {
    rect: Rect,
}

impl Panel for Combo {
    fn layout_info(&self, _ctx: &LayoutContext) -> LayoutInfo {
        LayoutInfo {
            place: LayoutPlace::Center,
            ..Default::default()
        }
    }

    fn set_rect(&mut self, rect: Rect) {
        self.set_rect(rect)
    }

    fn render(&self, ctx: &RenderPanelContext) -> Vec<UiPanel> {
        self.render(ctx.state)
    }
}

impl Combo {
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, state: &State) -> Vec<UiPanel> {
        let mode = state.config.state_value("mode");
        let show_combo = mode.is_some_and(|m| m.starts_with("combo-"));
        if !show_combo {
            return vec![];
        }

        let mut grid = GridStringBuilder::new();

        if let Some(keybinds_doc) = state.config.get("keybinds-doc") {
            for (i, (keybind, doc)) in keybinds_doc.iter().enumerate() {
                let y = i as _;
                grid.set_cell((0, y), Cell::new(format!("{keybind}: ")));
                // FIXME unecessary clone
                grid.set_cell((1, y), Cell::new(doc.first().cloned().unwrap_or_default()));
            }
        }

        let (inner_size, content) = grid.build();
        let size: Size = Size::new(inner_size.0 + 4, inner_size.1 + 2);
        let column = self.rect.width - size.column;
        let row = self.rect.height - size.row;
        let position = (column, row).into();

        let border_style = Style {
            foreground_color: state.config.get_theme_color("box-fg"),
            background_color: state.config.get_theme_color("box-bg"),
            ..Default::default()
        };
        let border_panel = decorated_rectangle(position, size, border_style, BORDER_ALL);

        let inner_style = Style {
            foreground_color: state.config.get_theme_color("editor-fg"),
            background_color: state.config.get_theme_color("box-bg"),
            ..Default::default()
        };

        let inner_panel = UiPanel {
            position: position.offset((2, 1)),
            size: inner_size.into(),
            content,
            spans: vec![StyledRegion {
                from: Position::ZERO,
                to: Position::new(size.column, size.row),
                priority: 2,
                style: inner_style,
            }],
        };

        vec![border_panel, inner_panel]
    }
}
