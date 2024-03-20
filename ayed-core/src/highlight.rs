use std::collections::HashMap;

use regex::Regex;

use crate::{
    text_buffer::TextBuffer,
    ui_state::{priority_from_str, Color, Style},
    utils::Position,
};

const DEFAULT_PRIORITY: u8 = 10;

#[derive(Debug, Default, Clone)]
pub struct Highlight {
    pub position: HighlightPosition,
    pub style: Style,
    pub priority: u8,
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

    let mut rules: Vec<(Vec<Regex>, Color, Option<u8>)> = Vec::new();
    for (rule_name, patterns) in syntax {
        let regexes = patterns
            .iter()
            .map(|pattern| Regex::new(pattern).unwrap())
            .collect::<Vec<_>>();
        let Some(values) = syntax_style.get(rule_name) else {
            continue;
        };
        let mut color = None;
        let mut priority = None;
        for value in values {
            if let Some(parsed_color) = Color::from_hex(value).ok() {
                color = Some(parsed_color);
            } else if let Ok(parsed_priority) = priority_from_str(value) {
                priority = Some(parsed_priority);
            }
        }
        let Some(color) = color else { continue };

        rules.push((regexes, color, priority));
    }

    for line_index in 0..buffer.line_count() {
        let Some(line) = buffer.line(line_index) else {
            break;
        };
        for (regexes, color, priority) in &rules {
            for regex in regexes {
                for capture in regex.captures_iter(&line) {
                    let matchh = capture
                        .get(1)
                        .unwrap_or(capture.get(0).expect("group 0 cannot fail"));
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
                        priority: priority.unwrap_or(DEFAULT_PRIORITY),
                    });
                }
            }
        }
    }

    highlights
}
