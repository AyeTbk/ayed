use std::collections::HashMap;

use regex::Regex;

use crate::{
    buffer::TextBuffer,
    ui_state::{Color, Style},
    utils::Position,
};

#[derive(Debug, Default, Clone)]
pub struct Highlight {
    pub position: HighlightPosition,
    pub style: Style,
    pub importance: u8,
}

#[derive(Debug, Clone)]
pub enum HighlightPosition {
    Panel { from: Position, to: Position },
    Content { from: Position, to: Position },
}

impl Default for HighlightPosition {
    fn default() -> Self {
        Self::Panel {
            from: Default::default(),
            to: Default::default(),
        }
    }
}

pub fn regex_syntax_highlight(
    buffer: &TextBuffer,
    syntax: &HashMap<String, Vec<String>>,
    syntax_style: &HashMap<String, Vec<String>>,
) -> Vec<Highlight> {
    let mut highlights = Vec::new();

    let mut rules: Vec<(Vec<Regex>, Color)> = Vec::new();
    for (rule_name, patterns) in syntax {
        let regexes = patterns
            .iter()
            .map(|pattern| Regex::new(pattern).unwrap())
            .collect::<Vec<_>>();
        let style_str = syntax_style.get(rule_name).and_then(|v| v.first());
        let Some(color) = style_str.and_then(|s| Color::from_hex(s).ok()) else {
            continue;
        };
        rules.push((regexes, color));
    }

    let mut line = String::new();
    for line_index in 0..buffer.line_count() {
        let Ok(()) = buffer.copy_line(line_index, &mut line) else {
            break;
        };
        for (regexes, color) in &rules {
            for regex in regexes {
                for capture in regex.captures_iter(&line) {
                    let matchh = capture.get(1).unwrap_or(capture.get(0).unwrap());
                    let match_chars_start = line
                        .char_indices()
                        .take_while(|(idx, _)| *idx != matchh.start())
                        .count() as u32;
                    let match_chars_count = line
                        .char_indices()
                        .skip_while(|(idx, _)| *idx != matchh.start())
                        .take_while(|(idx, _)| *idx != matchh.end())
                        .count() as u32;
                    let match_chars_end = (match_chars_start + match_chars_count).saturating_sub(1);

                    highlights.push(Highlight {
                        position: HighlightPosition::Content {
                            from: Position::new(match_chars_start, line_index),
                            to: Position::new(match_chars_end, line_index),
                        },
                        style: Style {
                            foreground_color: Some(*color),
                            ..Default::default()
                        },
                        importance: 10,
                    });
                }
            }
        }
    }

    highlights
}
