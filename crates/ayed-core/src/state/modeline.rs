use std::collections::HashMap;

use crate::{config::Config, ui::Style};

#[derive(Debug, Default, Clone)]
pub struct ModelineState {
    pub infos: Vec<ModelineInfo>,
    pub content_override: Option<ContentOverride>,
    pub histories: HashMap<String, PromptHistory>,
}

#[derive(Debug, Default, Clone)]
pub struct PromptHistory {
    pub entries: Vec<String>,
    pub selected_item: usize,
}

impl ModelineState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_message(&mut self, text: String) {
        self.content_override = Some(ContentOverride {
            text,
            ..Default::default()
        });
    }

    pub fn set_error(&mut self, text: String, config: &Config) {
        self.content_override = Some(ContentOverride {
            text,
            top_style: Some(Style {
                foreground_color: config.get_theme_color("modeline-text"),
                background_color: config.get_theme_color("error"),
                ..Default::default()
            }),
            bottom_style: Some(Style {
                foreground_color: config.get_theme_color("modeline-text"),
                background_color: config.get_theme_color("error-dark"),
                ..Default::default()
            }),
        });
    }

    pub fn clear_content_override(&mut self) {
        self.content_override = None;
    }

    pub fn iter(&self) -> impl Iterator<Item = &ModelineInfo> + '_ {
        self.infos.iter()
    }
}

#[derive(Debug, Clone)]
pub struct ModelineInfo {
    pub text: String,
    pub style: Style,
    pub align: Align,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

#[derive(Debug, Default, Clone)]
pub struct ContentOverride {
    pub text: String,
    pub top_style: Option<Style>,
    pub bottom_style: Option<Style>,
}
