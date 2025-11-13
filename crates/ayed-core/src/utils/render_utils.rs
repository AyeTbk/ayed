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

pub fn decorated_rectangle(
    position: Position,
    size: Size,
    style: Style,
    borders: BordersFlag,
) -> UiPanel {
    let mut panel = rectangle(position, size, style);
    let first_line_idx = 0;
    let last_line_idx = panel.content.len().saturating_sub(1);
    // let (h, v, tl, tr, bl, br) = ("─", '│', '┌', '┐', '└', '┘');
    // let (h, v, tl, tr, bl, br) = ("━", '┃', '┏', '┓', '┗', '┛');
    let (h, v, tl, tr, bl, br) = ("─", '│', '╭', '╮', '╰', '╯');
    for (i, line) in panel.content.iter_mut().enumerate() {
        let fill_row = (i == first_line_idx && borders & BORDER_TOP != 0)
            || (i == last_line_idx && borders & BORDER_BOTTOM != 0);
        if fill_row {
            *line = h.repeat(char_count(line));
        } else {
            if (borders & BORDER_LEFT) != 0 {
                line.pop();
                line.insert(0, v);
            }
            if (borders & BORDER_RIGHT) != 0 {
                line.pop();
                line.push(v);
            }
        }
        if i == first_line_idx {
            if (borders & BORDER_TOP) != 0 && (borders & BORDER_LEFT) != 0 {
                line.remove(0);
                line.insert(0, tl);
            }
            if (borders & BORDER_TOP) != 0 && (borders & BORDER_RIGHT) != 0 {
                line.pop();
                line.push(tr);
            }
        } else if i == last_line_idx {
            if (borders & BORDER_BOTTOM) != 0 && (borders & BORDER_LEFT) != 0 {
                line.remove(0);
                line.insert(0, bl);
            }
            if (borders & BORDER_BOTTOM) != 0 && (borders & BORDER_RIGHT) != 0 {
                line.pop();
                line.push(br);
            }
        }
    }
    panel
}

pub type BordersFlag = u8;
pub const BORDER_TOP: BordersFlag = 1;
pub const BORDER_BOTTOM: BordersFlag = 2;
pub const BORDER_LEFT: BordersFlag = 4;
pub const BORDER_RIGHT: BordersFlag = 8;
pub const BORDER_ALL: BordersFlag = 15;

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
