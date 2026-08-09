use crate::{
    panels::{Editor, LayoutContext, LayoutInfo, LayoutPlace, Panel, PanelContext, Sides},
    position::Position,
    selection::Selections,
    slotmap::Handle,
    state::{TextBuffer, View},
    ui::{
        Color, Rect, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::{
        render_utils::{BORDER_ALL, decorated_rectangle, separator_h},
        string_utils::line_clamped_filled,
    },
};

#[derive(Default)]
pub struct ListPicker {
    rect: Rect,
    view_handle: Option<Handle<View>>,
}

impl Panel for ListPicker {
    fn layout_info(&self, _ctx: &LayoutContext) -> LayoutInfo {
        LayoutInfo {
            place: LayoutPlace::FloatCenter,
            padding: Some(Sides {
                top: 2,
                bottom: 2,
                left: 6,
                right: 6,
            }),
            ..Default::default()
        }
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
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
        self.render(ctx)
    }

    fn name(&self) -> Option<&str> {
        Some("list-picker")
    }

    fn view(&self) -> Option<Handle<View>> {
        self.view_handle
    }
}

impl ListPicker {
    fn render(&self, ctx: &PanelContext) -> Vec<UiPanel> {
        let Some(view_handle) = self.view_handle else {
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

        let mut back_panel = decorated_rectangle(
            self.rect.top_left(),
            self.rect.size(),
            default_style,
            BORDER_ALL,
        );
        separator_h(2, &mut back_panel.content);

        let mut editor = Editor::with_view(view_handle);
        let editor_rect = Rect::from_positions(self.rect.top_left(), self.rect.top_right())
            .grown(0, 0, -2, -2)
            .offset((0, 1));
        editor.set_rect(editor_rect);
        let editor_panel = editor.render(ctx).remove(0);

        let list_rect = Rect::from_positions(self.rect.top_left(), self.rect.bottom_right())
            .grown(-3, -1, -2, -2);
        let list_panel = render_list_content(ctx, list_rect, default_style, text_color);

        vec![back_panel, editor_panel, list_panel]
    }
}

fn render_list_content(
    ctx: &PanelContext,
    rect: Rect,
    default_style: Style,
    text_color: Option<Color>,
) -> UiPanel {
    let size = rect.size();
    let mut content = Vec::new();
    let mut spans = Vec::new();

    let list_is_empty = ctx.state.list_picker.items.is_empty();

    for y in 0..size.row {
        let mut style = default_style;
        if !list_is_empty && y as usize == ctx.state.list_picker.selected_item {
            style.invert = true;
        }
        let text = if list_is_empty && y == 0 {
            style.foreground_color = Some(Color::rgb(112, 112, 112));
            "nothing to see here"
        } else if let Some(item) = ctx.state.list_picker.items.get(y as usize) {
            if item.kind == ListPickerItemKind::Section {
                style.foreground_color = Some(Color::rgb(112, 112, 112));
                style.bold = true;
            } else {
                style.foreground_color = text_color;
            }

            item.label
                .split_once('\n')
                .map(|(l, _)| l)
                .unwrap_or(&item.label)
        } else {
            ""
        };
        let line = line_clamped_filled(text, 0, size.column as usize, ' ');
        content.push(line);

        spans.push(StyledRegion {
            from: Position::new(0, y as i32),
            to: Position::new(size.column as i32, y as i32),
            style,
            priority: 2,
        });
    }

    UiPanel {
        position: rect.top_left(),
        size,
        content,
        spans,
    }
}

#[derive(Default)]
pub struct ListPickerState {
    pub raw_items: Vec<ListPickerItem>, // All items
    pub items: Vec<ListPickerItem>,     // Filtered items
    pub selected_item: usize,
}

impl ListPickerState {
    pub fn select_next(&mut self) {
        self.select_impl(1);
    }

    pub fn select_previous(&mut self) {
        self.select_impl(-1);
    }

    pub fn reselect(&mut self) {
        self.selected_item = 0;
        if self.items.is_empty() {
            return;
        }
        let selected_item = self.items.get(self.selected_item);
        if let Some(item) = selected_item
            && item.kind == ListPickerItemKind::Section
        {
            self.select_next();
        }
    }

    fn select_impl(&mut self, direction: i32) {
        if self.items.is_empty() {
            return;
        }

        let dir = direction.signum();
        let mut i = self.selected_item as i32 + dir;
        loop {
            i = i32::rem_euclid(i, self.items.len() as i32);
            if i == self.selected_item as i32 {
                // Couldn't find anything
                break;
            }
            let item = &self.items[i as usize];
            if item.kind == ListPickerItemKind::Item {
                self.selected_item = i as usize;
                break;
            }
            i += dir;
        }
    }
}

#[derive(Clone)]
pub struct ListPickerItem {
    pub kind: ListPickerItemKind,
    pub label: String,
    pub command: String,
    pub filter_text: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ListPickerItemKind {
    Section,
    Item,
}
