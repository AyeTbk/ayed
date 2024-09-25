use std::sync::LazyLock;

use regex::Regex;

use crate::{
    command::ExecuteCommandContext,
    position::Position,
    slotmap::Handle,
    state::{State, View},
    ui::{
        ui_state::{StyledRegion, UiPanel},
        Color, Rect, Style,
    },
    utils::string_utils::char_count,
};

const FOREGROUND_COLOR: Color = Color::rgb(128, 128, 128);
const CODE_FOREGROUND_COLOR: Color = Color::rgb(250, 120, 120);

#[derive(Default)]
pub struct Warpdrive {
    rect: Rect,
    state: Option<WarpdriveState>,
}

impl Warpdrive {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn clear_state(&mut self) {
        self.state = None;
    }

    pub fn render(&self, _state: &State) -> Option<UiPanel> {
        let position = self.rect.top_left();
        let size = self.rect.size();
        let mut spans: Vec<StyledRegion> = (0..size.row)
            .map(|row| StyledRegion {
                from: Position::new(0, row as _),
                to: Position::new(size.column as _, row as _),
                priority: 1,
                style: Style {
                    foreground_color: Some(FOREGROUND_COLOR),
                    background_color: None,
                    ..Default::default()
                },
            })
            .collect();

        for jump_point in self.state.as_ref()?.jump_points.iter() {
            spans.push(StyledRegion {
                from: jump_point.start_in_view,
                to: jump_point
                    .start_in_view
                    .offset(((jump_point.code.len() - 1) as _, 0)),
                priority: 56,
                style: Style {
                    foreground_color: Some(CODE_FOREGROUND_COLOR),
                    background_color: None,
                    ..Default::default()
                },
            });
        }

        Some(UiPanel {
            position,
            size,
            content: self.state.as_ref()?.content.clone(),
            spans,
        })
    }
}

struct WarpdriveState {
    content: Vec<String>,
    input: String,
    jump_points: Vec<JumpPoint>,
}

static REGEX_JUMP_POINT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+").unwrap());

impl WarpdriveState {
    pub fn new(ctx: &ExecuteCommandContext, view_handle: Handle<View>) -> WarpdriveState {
        // TODO allow providing the regex for jump points

        let view = ctx.state.views.get(view_handle);
        let mut content = ctx.panels.editor.render(&ctx.state).content;

        // Gather jump points
        let mut jump_points = Vec::new();
        let mut jump_points_indices = Vec::new();
        for (i, line) in content.iter().enumerate() {
            let row = i as u32;
            for matsh in REGEX_JUMP_POINT.find_iter(&line) {
                let start_column = char_count(&line[..matsh.start()]);
                let end_column = char_count(&line[..matsh.end()]).saturating_sub(1);
                let start_in_view = Position::new(start_column, row);
                let end_in_view = Position::new(end_column, row);
                let Some(start) = view.map_view_position_to_true_position(start_in_view) else {
                    continue;
                };
                let Some(end) = view.map_view_position_to_true_position(end_in_view) else {
                    continue;
                };
                let code = String::new();
                jump_points_indices.push((i, (matsh.start(), matsh.end())));
                jump_points.push(JumpPoint {
                    code,
                    start_in_view,
                    start,
                    end,
                });
            }
        }

        // Assign jump points codes
        fn assign_codes_recursive<A>(
            jump_points: &mut [JumpPoint],
            code_prefix: &[char],
            alphabet: A,
        ) where
            A: Iterator<Item = char> + Clone,
        {
            if jump_points.len() == 0 {
                return;
            }
            let alphabet_size = alphabet.clone().count();
            let jump_points_per_letter = ((jump_points.len() - 1) / alphabet_size) + 1;
            let mut letters = alphabet.clone();
            if jump_points_per_letter > 1 {
                for jump_points_chunk in jump_points.chunks_mut(jump_points_per_letter) {
                    let mut code = code_prefix.to_vec();
                    code.push(letters.next().expect("jump points should have been split in chunks such that there are fewer chunks than letters in alphabet"));
                    assign_codes_recursive(jump_points_chunk, &code, alphabet.clone());
                }
            } else {
                for JumpPoint { code, .. } in jump_points {
                    *code = String::new();
                    code.extend(code_prefix);
                    code.push(letters.next().expect("jump points should have been split in chunks such that there are fewer jump points than letters in alphabet"));
                }
            }
        }
        let alphabet = 'a'..='z';
        assign_codes_recursive(&mut jump_points, &[], alphabet);

        // Insert codes in content
        for (jump_point, indices) in jump_points.iter().zip(jump_points_indices.iter()) {
            let line = &mut content[indices.0];
            let replace_bytes_size: usize = line[indices.1 .0..]
                .chars()
                .take(jump_point.code.len())
                .map(char::len_utf8)
                .sum();
            let replace_range = (indices.1 .0)..((indices.1 .0) + replace_bytes_size);
            line.replace_range(replace_range, &jump_point.code);
        }

        Self {
            content,
            input: Default::default(),
            jump_points,
        }
    }

    pub fn input(&mut self, ch: char) -> WarpdriveInputResult {
        self.input.push(ch);
        self.jump_points = std::mem::take(&mut self.jump_points)
            .into_iter()
            .filter(|jp| jp.code.starts_with(&self.input))
            .collect();
        if self.jump_points.len() == 1 {
            let JumpPoint { start, end, .. } = self.jump_points.pop().expect("len is 1");
            WarpdriveInputResult::Finished((start, end))
        } else if self.jump_points.len() == 0 {
            WarpdriveInputResult::FinishedEmpty
        } else {
            WarpdriveInputResult::Unfinished
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WarpdriveInputResult {
    Unfinished,
    FinishedEmpty,
    Finished((Position, Position)),
}

#[derive(Debug)]
struct JumpPoint {
    code: String,
    start_in_view: Position,
    start: Position,
    end: Position,
}

pub mod commands {
    use crate::{
        command::CommandRegistry,
        event::EventRegistry,
        selection::{Selection, Selections},
    };

    use super::{WarpdriveInputResult, WarpdriveState};

    pub fn register_warpdrive_commands(cr: &mut CommandRegistry, _ev: &mut EventRegistry) {
        cr.register("warpdrive", |_opt, ctx| {
            let Some(view_handle) = ctx.state.focused_view() else {
                return Ok(());
            };

            ctx.panels.warpdrive.state = Some(WarpdriveState::new(&ctx, view_handle));

            ctx.queue.push("panel-focus warpdrive");
            Ok(())
        });

        cr.register("warpdrive-input", |opt, ctx| {
            let Some(state) = ctx.panels.warpdrive.state.as_mut() else {
                return Ok(());
            };

            let ch = opt.chars().next().unwrap_or_default();
            match state.input(ch) {
                WarpdriveInputResult::Finished((start, end)) => {
                    let selections = Selections::new_with(
                        Selection::new().with_anchor(start).with_cursor(end),
                        &[],
                    );
                    ctx.queue.push(format!("selections-set {selections}"));
                    ctx.queue.push("panel-focus editor");
                }
                WarpdriveInputResult::FinishedEmpty => {
                    ctx.queue.push("panel-focus editor");
                }
                _ => {}
            }

            Ok(())
        });
    }
}
