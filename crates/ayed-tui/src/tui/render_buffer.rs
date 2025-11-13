use ayed_core::ui::{Size, Style, ui_state::UiState};

pub struct RenderBuffer {
    pub buffer: Vec<Vec<RenderBufferCell>>,
}

#[derive(Debug, Default, Clone)]
pub struct RenderBufferCell {
    pub data: char,
    pub style: Style,
}

impl RenderBuffer {
    pub fn render(viewport_size: Size, ui_state: UiState) -> Self {
        let (viewport_width, viewport_height) =
            (viewport_size.column as i32, viewport_size.row as i32);
        let empty_cell = RenderBufferCell {
            data: ' ',
            style: Default::default(),
        };
        let mut buffer = vec![vec![empty_cell; viewport_width as usize]; viewport_height as usize];

        for mut panel in ui_state.panels.into_iter() {
            panel.prepare_for_render();

            let start_y = panel.position.row;
            let after_end_y = start_y + panel.size.row as i32;
            let start_x = panel.position.column;
            let after_end_x = start_x + panel.size.column as i32;

            for (y, line) in (start_y..after_end_y).zip(panel.content.iter()) {
                if y < 0 || y >= viewport_height {
                    continue;
                }

                let panel_local_row = y - panel.position.row;

                for (x, ch) in (start_x..after_end_x).zip(line.chars()) {
                    if x < 0 || x >= viewport_width {
                        continue;
                    }

                    let panel_local_column = x - panel.position.column;
                    let panel_local_pos = (panel_local_column, panel_local_row);
                    let style = panel.style_for_pos(panel_local_pos.into());

                    buffer[y as usize][x as usize] = RenderBufferCell { data: ch, style }
                }
            }
        }

        Self { buffer }
    }
}
