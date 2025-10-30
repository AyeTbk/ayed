use crate::{
    position::Position,
    ui::{
        Color, Rect, Style,
        theme::colors::ACCENT,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::line_clamped_filled,
};

use super::{Editor, FocusedPanel, RenderPanelContext};

#[derive(Default)]
pub struct FilePicker {
    rect: Rect,
}

impl FilePicker {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &RenderPanelContext) -> Vec<UiPanel> {
        let FocusedPanel::FilePicker(view_handle) = ctx.state.focused_panel else {
            return Vec::new();
        };

        let size = self.rect.size();
        let mut spans = Vec::new();
        let mut content = Vec::new();

        for y in 0..size.row {
            content.push(" ".repeat(size.column as usize));
            spans.push(StyledRegion {
                from: Position::new(0, y as i32),
                to: Position::new(size.column as i32, y as i32),
                style: Style {
                    background_color: Some(ACCENT),
                    ..Default::default()
                },
                priority: 1,
            });
        }

        let mut editor = Editor::with_view(view_handle);
        let editor_rect = Rect::from_positions(self.rect.top_left(), self.rect.top_right())
            .grown(0, 0, -2, -2)
            .offset((0, 1));
        editor.set_rect(editor_rect);
        let editor_panel = editor.render(ctx);

        let list_rect = Rect::from_positions(self.rect.top_left(), self.rect.bottom_right())
            .grown(-3, -1, -2, -2);
        let list_panel = render_file_list(ctx, list_rect);

        vec![
            UiPanel {
                position: self.rect.top_left(),
                size,
                content: content,
                spans,
            },
            editor_panel,
            list_panel,
        ]
    }
}

fn render_file_list(ctx: &RenderPanelContext, rect: Rect) -> UiPanel {
    let size = rect.size();
    let mut content = Vec::new();
    let mut spans = Vec::new();

    let file_list_is_empty = ctx.state.file_picker.list_items.is_empty();

    for y in 0..size.row {
        let mut style = Style::default();
        if !file_list_is_empty && y as usize == ctx.state.file_picker.selected_item {
            style.invert = true;
        }
        let text = if file_list_is_empty && y == 0 {
            style.foreground_color = Some(Color::rgb(112, 112, 112));
            "no such file"
        } else if let Some(item) = ctx.state.file_picker.list_items.get(y as usize) {
            if matches!(item, FileListItem::Section { .. }) {
                style.foreground_color = Some(Color::rgb(112, 112, 112));
                style.bold = true;
            }

            item.text()
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

pub enum FileListItem {
    Section { text: String },
    File { text: String, path: String },
}

impl FileListItem {
    pub fn text(&self) -> &str {
        match self {
            Self::Section { text } => text,
            Self::File { text, .. } => text,
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            Self::Section { .. } => None,
            Self::File { path, .. } => Some(path),
        }
    }
}

#[derive(Default)]
pub struct FilePickerState {
    pub list_items: Vec<FileListItem>,
    pub selected_item: usize,
}

impl FilePickerState {
    pub fn select_next(&mut self) {
        self.select_impl(1);
    }

    pub fn select_previous(&mut self) {
        self.select_impl(-1);
    }

    pub fn reselect(&mut self) {
        self.selected_item = 0;
        self.select_next();
        self.select_previous();
    }

    fn select_impl(&mut self, direction: i32) {
        if self.list_items.is_empty() {
            return;
        }

        let dir = direction.signum();
        let mut i = self.selected_item as i32 + dir;
        loop {
            i = i32::rem_euclid(i, self.list_items.len() as i32);
            if i == self.selected_item as i32 {
                // Couldn't find anything
                break;
            }
            let item = &self.list_items[i as usize];
            if matches!(item, FileListItem::File { .. }) {
                self.selected_item = i as usize;
                break;
            }
            i += dir;
        }
    }
}

pub mod commands {
    use crate::{
        command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
        panels::file_picker::FileListItem,
    };

    pub fn register_file_picker_commands(cr: &mut CommandRegistry) {
        cr.register(
            "file-picker-confirm",
            focused_buffer_command(|_opt, ctx| {
                let idx = ctx.state.file_picker.selected_item;
                let Some(item) = ctx.state.file_picker.list_items.get(idx) else {
                    return Ok(());
                };
                let Some(path) = item.path() else { return Ok(()) };
                if path.trim() == "" {
                    return Ok(());
                }

                ctx.queue.push(format!("edit {path}"));
                ctx.queue.push("panel-focus editor");

                Ok(())
            }),
        );

        cr.register("file-picker-select", |opt, ctx| {
            let opts = Options::new().flag("next").flag("previous").parse(opt)?;
            let next = opts.contains("next");
            let previous = opts.contains("previous");

            if next {
                ctx.state.file_picker.select_next();
            }
            if previous {
                ctx.state.file_picker.select_previous();
            }

            Ok(())
        });

        cr.register(
            "file-picker-fill-list",
            focused_buffer_command(|_opt, ctx| {
                let filter = ctx.buffer.line(0).unwrap_or_default();
                match file_picker_fill_list(filter) {
                    Ok(list) => ctx.state.file_picker.list_items = list,
                    Err(err) => return Err(err.to_string()),
                }
                ctx.state.file_picker.reselect();
                Ok(())
            }),
        );
    }

    fn file_picker_fill_list(filter: &str) -> std::io::Result<Vec<FileListItem>> {
        fn aux(filter: &str, dir_path: &str, list: &mut Vec<FileListItem>) -> std::io::Result<()> {
            if list.len() > 200 {
                return Ok(());
            }
            for maybe_entry in std::fs::read_dir(dir_path)? {
                let Ok(entry) = maybe_entry else { continue };
                let path = entry.path().to_str().unwrap().to_string();
                if entry.file_type()?.is_dir() {
                    aux(filter, &path, list)?;
                } else {
                    if !path.contains(filter) {
                        continue;
                    }
                    list.push(FileListItem::File {
                        text: path.clone(),
                        path: path,
                    });
                }
            }
            Ok(())
        }

        let mut list = Vec::new();
        aux(filter, ".", &mut list)?;
        Ok(list)
    }
}
