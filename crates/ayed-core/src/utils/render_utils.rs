use crate::{
    position::Position,
    ui::{
        Size, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::char_count,
};

pub fn rectangle(position: Position, size: Size, style: Style) -> UiPanel {
    let mut content = Vec::new();
    let mut spans = Vec::new();
    let line = " ".repeat(size.column as _);
    for y in 0..size.row as _ {
        content.push(line.clone());
        spans.push(StyledRegion {
            from: Position::new(0, y as _),
            to: Position::new(size.column as _, y as _),
            priority: 1,
            style,
        });
    }
    UiPanel {
        position,
        size,
        content,
        spans,
    }
}

pub fn decorated_rectangle(position: Position, size: Size, style: Style) -> UiPanel {
    let mut panel = rectangle(position, size, style);
    let first_line_idx = 0;
    let last_line_idx = panel.content.len().saturating_sub(1);
    let (h, v, tl, tr, bl, br) = ("─", '│', '┌', '┐', '└', '┘');
    for (i, line) in panel.content.iter_mut().enumerate() {
        if i == first_line_idx || i == last_line_idx {
            *line = h.repeat(char_count(line));
        } else {
            (line.pop(), line.pop());
            line.insert(0, v);
            line.push(v);
        }
        if i == first_line_idx {
            (line.pop(), line.pop());
            line.insert(0, tl);
            line.push(tr);
        } else if i == last_line_idx {
            (line.pop(), line.pop());
            line.insert(0, bl);
            line.push(br);
        }
    }
    panel
}

pub fn separator_h(row: i32, content: &mut Vec<String>) {
    let Some(line) = content.get_mut(row as usize) else { return };
    let line_len = char_count(line);
    if line_len <= 2 {
        return;
    }
    let sep = "─".repeat(line_len.saturating_sub(2));
    let mut chars = line.chars();
    let first = chars.next().unwrap();
    let last = chars.last().unwrap();
    *line = format!("{first}{sep}{last}");
}
