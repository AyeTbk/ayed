use std::any::Any;
use std::collections::HashMap;

use crate::slotmap::Handle;
use crate::state::{Resources, State, View};
use crate::ui::ui_state::UiPanel;
use crate::ui::{Rect, Size};

mod editor;
pub use self::editor::Editor;

pub mod prompt;
pub use self::prompt::Prompt;

pub mod status_bar;
pub use self::status_bar::StatusBar;

pub mod list_picker;
pub use self::list_picker::ListPicker;

pub mod hover_info;
pub use self::hover_info::HoverInfo;

pub mod warpdrive;
pub use self::warpdrive::Warpdrive;

mod combo;
pub use self::combo::Combo;

mod completions;
pub use self::completions::Completions;

pub struct Panels {
    pub panels: Vec<Box<dyn Panel>>,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            panels: vec![
                Box::new(Editor::default().with_line_numbers()),
                Box::new(Prompt::default()),
                Box::new(StatusBar::default()),
                Box::new(Warpdrive::default()),
                Box::new(ListPicker::default()),
                Box::new(HoverInfo::default()),
                Box::new(Combo::default()),
                Box::new(Completions::default()),
            ],
        }
    }
}

impl Panels {
    pub fn compute_layout(&mut self, viewport_size: Size, state: &State, resources: &Resources) {
        compute_layout(viewport_size, state, resources, &mut self.panels);
    }

    pub fn render(&self, ctx: &PanelContext) -> Vec<UiPanel> {
        self.panels
            .iter()
            .map(|p| p.render(ctx))
            .flatten()
            .collect()
    }

    pub fn panel_with_name(&self, name: &str) -> Option<&dyn Panel> {
        // FIXME ugly hack because focused_panel is initialized empty.
        let mut name = name;
        if name == "" {
            name = "editor"
        }

        self.panels
            .iter()
            .filter_map(|p| {
                if p.name() == Some(name) {
                    Some(p.as_ref())
                } else {
                    None
                }
            })
            .next()
    }

    pub fn panel_with_name_mut(&mut self, name: &str) -> Option<&mut dyn Panel> {
        self.panels
            .iter_mut()
            .filter_map(|p| {
                if p.name() == Some(name) {
                    Some(p.as_mut())
                } else {
                    None
                }
            })
            .next()
    }

    pub fn panel_of_type<T: 'static>(&self) -> Option<&T> {
        for panel in &self.panels {
            let any = panel.as_ref() as &dyn Any;
            if let Some(t) = any.downcast_ref::<T>() {
                return Some(t);
            }
        }
        None
    }

    pub fn panel_of_type_mut<T: 'static>(&mut self) -> Option<&mut T> {
        for panel in &mut self.panels {
            let any = panel.as_mut() as &mut dyn Any;
            if let Some(t) = any.downcast_mut::<T>() {
                return Some(t);
            }
        }
        None
    }
}

pub struct PanelContext<'a> {
    pub state: &'a mut State,
    pub resources: &'a mut Resources,
}

pub trait Panel: Any {
    fn layout_info(&self, ctx: &LayoutContext) -> LayoutInfo;
    fn rect(&self) -> Rect;
    fn set_rect(&mut self, rect: Rect);
    fn render(&self, ctx: &PanelContext) -> Vec<UiPanel>;

    fn content_rect(&self) -> Rect {
        self.rect()
    }

    fn on_focus(&mut self, _ctx: &mut PanelContext) {}
    fn on_unfocus(&mut self, _ctx: &mut PanelContext) {}

    fn name(&self) -> Option<&str> {
        None
    }

    fn enabled(&self) -> bool {
        true
    }

    fn view(&self) -> Option<Handle<View>> {
        None
    }
}

pub struct LayoutContext<'a> {
    pub state: &'a State,
    pub resources: &'a Resources,
    pub full_viewport_size: Size,
}

#[derive(Debug, Default)]
pub struct LayoutInfo {
    pub place: LayoutPlace,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub padding: Option<Sides<i32>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutPlace {
    #[default]
    Center,
    South,
    FloatTop,
    FloatCenter,
    FloatBottom,
    FloatBottomRight,
    ManuallyManaged,
}

#[derive(Debug, Default)]
pub struct Sides<T> {
    pub top: T,
    pub bottom: T,
    pub left: T,
    pub right: T,
}

pub fn compute_layout(
    viewport_size: Size,
    state: &State,
    resources: &Resources,
    panels: &mut Vec<Box<dyn Panel>>,
) {
    let mut places: HashMap<LayoutPlace, Vec<(LayoutInfo, &mut dyn Panel)>> = Default::default();
    let ctx = LayoutContext {
        state,
        resources,
        full_viewport_size: viewport_size,
    };
    for panel in panels {
        if !panel.enabled() {
            continue;
        }

        let info = panel.layout_info(&ctx);
        places
            .entry(info.place)
            .or_default()
            .push((info, Box::as_mut(panel)));
    }

    let mut viewport = Rect::new(0, 0, viewport_size.column, viewport_size.row);
    let layout_ordered_places = [
        LayoutPlace::South,
        LayoutPlace::FloatTop,
        LayoutPlace::FloatCenter,
        LayoutPlace::FloatBottom,
        LayoutPlace::FloatBottomRight,
        LayoutPlace::Center,
    ];
    for place in &layout_ordered_places {
        let Some(panels_infos) = places.get_mut(place) else { continue };

        for (info, panel) in panels_infos {
            let (h_expand, v_expand) = match place {
                LayoutPlace::South | LayoutPlace::FloatBottom | LayoutPlace::FloatTop => {
                    (true, false)
                }
                LayoutPlace::Center | LayoutPlace::FloatCenter => (true, true),
                _ => (false, false),
            };
            let width_fallback = if h_expand { viewport.width } else { 1 };
            let height_fallback = if v_expand { viewport.height } else { 1 };
            let width = info.width.unwrap_or(width_fallback);
            let height = info.height.unwrap_or(height_fallback);
            let (top, left);
            match place {
                LayoutPlace::FloatTop => {
                    top = viewport.top() + 1;
                    left = viewport.left() + (viewport.width - width) / 2;
                }
                LayoutPlace::South | LayoutPlace::FloatBottom => {
                    top = viewport.bottom() - height + 1;
                    left = 0;
                }
                LayoutPlace::Center | LayoutPlace::FloatCenter => {
                    top = viewport.top() + (viewport.height - height) / 2;
                    left = viewport.left() + (viewport.width - width) / 2;
                }
                _ => {
                    let vpad = 1;
                    let hpad = 2;
                    top = viewport.bottom() - (viewport.height - height) - vpad;
                    left = viewport.right() - (viewport.width - width) - hpad;
                }
            }

            let mut rect = Rect::new(left, top, width, height);
            if let Some(pad) = info.padding.as_ref() {
                rect = rect.grown(-pad.top, -pad.bottom, -pad.left, -pad.right);
            }

            match place {
                LayoutPlace::South => {
                    viewport = viewport.grown(0, -rect.height, 0, 0);
                }
                _ => {}
            }

            panel.set_rect(rect);
        }
    }
}
