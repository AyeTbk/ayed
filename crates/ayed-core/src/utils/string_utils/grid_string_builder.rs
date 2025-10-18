use std::collections::BTreeMap;

use crate::utils::string_utils::char_count;

pub struct GridStringBuilder {
    cells: BTreeMap<CellId, Cell>,
    spans: Vec<(CellId, CellId)>,
    max_cell_id: CellId,
}

impl GridStringBuilder {
    pub fn new() -> Self {
        GridStringBuilder {
            cells: Default::default(),
            spans: Default::default(),
            max_cell_id: Default::default(),
        }
    }

    pub fn cell(&self, id: impl Into<CellId>) -> Option<&Cell> {
        self.cell_and_id(id).map(|(_, cell)| cell)
    }

    pub fn cell_and_id(&self, id: impl Into<CellId>) -> Option<(CellId, &Cell)> {
        let actual_id = self.get_span_aware_cell_id(id.into());
        self.cells.get(&actual_id).map(|cell| (actual_id, cell))
    }

    pub fn set_cell(&mut self, id: impl Into<CellId>, cell: Cell) {
        let id = id.into();
        self.max_cell_id = self.max_cell_id.min_max(&id).1;
        self.cells.insert(id.into(), cell);
    }

    pub fn set_cell_span(&mut self, from: impl Into<CellId>, to: impl Into<CellId>) {
        let from = from.into();
        let to = to.into();
        let (min, max) = from.min_max(&to);
        self.spans.push((min, max));
    }

    pub fn columns(&self) -> impl Iterator<Item = impl Iterator<Item = (CellId, &Cell)> + '_> + '_ {
        let width = self.max_cell_id.x;
        let height = self.max_cell_id.y;

        (0..=width).map(move |x| {
            (0..=height)
                .map(move |y| CellId { x, y })
                .filter_map(|id| self.cell(id).map(|cell| (id, cell)))
        })
    }

    pub fn build(self) -> ((u32, u32), Vec<String>) {
        let grid_char_height = self.max_cell_id.y as usize + 1;
        let column_char_widths: Vec<usize> = self
            .columns()
            .map(|column| {
                column
                    .map(|(id, cell)| {
                        if self.is_cell_end_of_its_span(id) {
                            char_count(&cell.content)
                        } else {
                            0
                        }
                    })
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let grid_char_width: usize = column_char_widths.iter().sum();

        let column_data = column_char_widths
            .iter()
            .enumerate()
            .map(|(x, column_width)| {
                let start_column: usize = column_char_widths
                    .iter()
                    .enumerate()
                    .take_while(|(i, _)| *i < x)
                    .map(|(_, width)| width)
                    .sum();
                (x, start_column, *column_width)
            });

        let mut grid = vec![String::new(); grid_char_height];
        for (y, buf) in grid.iter_mut().enumerate() {
            for (x, column_start, column_width) in column_data.clone() {
                let id: CellId = (x as _, y as _).into();
                if let Some((actual_id, cell)) = self.cell_and_id(id) {
                    if id == actual_id {
                        buf.push_str(&cell.content);
                    }
                }

                let column_end = column_start + column_width;
                let padding_len = column_end.saturating_sub(char_count(&buf));
                for _ in 0..padding_len {
                    buf.push(' ');
                }
            }
        }
        ((grid_char_width as _, grid_char_height as _), grid)
    }

    fn get_span_aware_cell_id(&self, id: CellId) -> CellId {
        self.get_cell_span(id).0
    }

    fn get_cell_span(&self, id: CellId) -> (CellId, CellId) {
        for (from, to) in &self.spans {
            if (from.x <= id.x && id.x <= to.x) && (from.y <= id.y && id.y <= to.y) {
                return (*from, *to);
            }
        }
        (id, id)
    }

    fn is_cell_end_of_its_span(&self, id: CellId) -> bool {
        self.get_cell_span(id).1 == id
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellId {
    x: u16,
    y: u16,
}

impl CellId {
    pub fn min_max(&self, other: &Self) -> (Self, Self) {
        let a = *self;
        let b = *other;
        let min = CellId {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
        };
        let max = CellId {
            x: a.x.max(b.x),
            y: a.y.max(b.y),
        };
        (min, max)
    }
}

impl From<(u16, u16)> for CellId {
    fn from(value: (u16, u16)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

#[derive(Debug)]
pub struct Cell {
    pub content: String,
}

impl Cell {
    pub fn new(content: impl Into<String>) -> Self {
        Cell {
            content: content.into(),
        }
    }

    pub fn new_empty() -> Self {
        Cell {
            content: String::new(),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Cell::new_empty()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_build() {
        let mut grid = GridStringBuilder::new();

        grid.set_cell((0, 0), Cell::new("1: "));
        grid.set_cell((1, 0), Cell::new("hello"));
        grid.set_cell((0, 1), Cell::new("2: "));
        grid.set_cell((1, 1), Cell::new("bye"));

        let (size, content) = grid.build();

        assert_eq!(content[0], "1: hello");
        assert_eq!(content[1], "2: bye  ");
        assert_eq!(size, (8, 2));
    }
}
