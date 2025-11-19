use ayed_core::ui::{
    Rect, Size, Style,
    ui_state::{UiPanel, UiState},
};

pub struct RenderBuffer {
    pub buffer: Vec<Vec<RenderBufferCell>>,
}

#[derive(Debug, Default, Clone)]
pub struct RenderBufferCell {
    pub data: char,
    pub style: Style,
    pub panel_idx: usize,
}

impl RenderBuffer {
    pub fn render(viewport_size: Size, ui_state: UiState) -> Self {
        let (buffer_width, buffer_height) =
            (viewport_size.column as usize, viewport_size.row as usize);
        let empty_cell = RenderBufferCell {
            data: ' ',
            style: Default::default(),
            panel_idx: 0,
        };
        let mut buffer = vec![vec![empty_cell; buffer_width]; buffer_height];

        for (idx, mut panel) in ui_state.panels.into_iter().enumerate() {
            panel.prepare_for_render();

            Self::gather_styles(&mut buffer, idx, &panel, viewport_size);
            Self::render_panel(&mut buffer, &panel, viewport_size);
        }

        Self { buffer }
    }

    fn gather_styles(
        buffer: &mut Vec<Vec<RenderBufferCell>>,
        panel_idx: usize,
        panel: &UiPanel,
        buffer_size: Size,
    ) {
        let buffer_rect = Rect::with_position_and_size((0, 0).into(), buffer_size);
        for sr in &panel.spans {
            let panel_rect = Rect::with_position_and_size(panel.position, panel.size);
            let styled_rect = Rect::from_positions(sr.from, sr.to).offset(panel.position);
            let Some(confied_styled_rect) = styled_rect.intersection(panel_rect) else {
                continue;
            };
            let Some(rect) = buffer_rect.intersection(confied_styled_rect) else {
                continue;
            };

            for rect_cell in rect.cells() {
                let y = rect_cell.row as usize;
                let x = rect_cell.column as usize;

                let is_new_for_panel = panel_idx != buffer[y][x].panel_idx;
                buffer[y][x].panel_idx = panel_idx;

                let style = &mut buffer[y][x].style;

                if style.foreground_color.is_none() || is_new_for_panel {
                    style.foreground_color = sr.style.foreground_color
                }
                if style.background_color.is_none() || is_new_for_panel {
                    style.background_color = sr.style.background_color
                }
                if !style.invert || is_new_for_panel {
                    style.invert = sr.style.invert
                }
                if !style.bold || is_new_for_panel {
                    style.bold = sr.style.bold
                }
                if !style.underlined || is_new_for_panel {
                    style.underlined = sr.style.underlined
                }
            }
        }
    }

    fn render_panel(buffer: &mut Vec<Vec<RenderBufferCell>>, panel: &UiPanel, buffer_size: Size) {
        let start_y = panel.position.row;
        let after_end_y = start_y + panel.size.row;
        let start_x = panel.position.column;
        let after_end_x = start_x + panel.size.column;

        for (y, line) in (start_y..after_end_y).zip(panel.content.iter()) {
            if y < 0 || y >= buffer_size.row {
                continue;
            }

            for (x, ch) in (start_x..after_end_x).zip(line.chars()) {
                if x < 0 || x >= buffer_size.column {
                    continue;
                }

                buffer[y as usize][x as usize].data = ch;
            }
        }
    }
}
