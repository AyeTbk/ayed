use std::collections::HashMap;

use log::debug;
use regex::Regex;

use crate::{
    config::applied_config::{AppliedConfig, EditorConfig, MappingEntries},
    input::Input,
    ui::{Color, style::SyntaxStyle},
};

mod applied_config;

pub mod insert_order_map;

// NOTES
//
// ast:
//      Abstract form of the written config module.
//
// ConfigModule:
//      Config module processed such as to mostly be a list of conditionnal
//      mappings, each with their "flattened" selectors stack, and with mixins
//      and layers applied.
//      Substitutions have not been applied yet.
//
// AppliedConfig:
//      All config modules flattened to a map of unconditional mappings with
//      substitutions in entries resolved, determined by the config state at
//      the time of building (applying the state to the config).
//      Also applies substitutions at the very start of the applying
//      process.
//
// command:
//      Registered command invocation. May need substitution at the point of
//      invocation for "arg". Maybe use $($) for it?

#[derive(Default)]
pub struct Config {
    modules: Vec<ConfigModule>,
    state: ConfigState,
    current_config: AppliedConfig,
}

impl Config {
    pub fn add_module(&mut self, src: &str) -> Result<(), ()> {
        let module = parse_module(src)?;
        self.modules.push(module);
        self.rebuild_current_config();

        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&MappingEntries<Vec<String>>> {
        self.current_config.get(key)
    }

    pub fn get_entry_value(&self, mapping: &str, entry_name: &str) -> Result<&str, String> {
        self.get(mapping)
            .and_then(|m| m.get(entry_name))
            .and_then(|e| e.as_slice().first())
            .map(String::as_str)
            .ok_or_else(|| format!("entry not found '{entry_name}' of '{mapping}'"))
    }

    pub fn state_value(&self, state_name: &str) -> Option<&str> {
        self.state.get(state_name)
    }

    /// DO NOT CALL.
    /// You probably want CommandQueue::set_state(...).
    /// Should only be called by the "state-set" command, otherwise
    /// state-set hooks won't work.
    pub fn set_state(&mut self, state_name: impl Into<String>, value: impl Into<String>) {
        self.state.set(state_name, value);
        // TODO rebuild more efficiently instead of rebuilding completely
        self.rebuild_current_config();
    }

    pub fn get_keybind(&self, input: Input) -> Option<&[String]> {
        // TODO have a map of actual inputs in the Applied config instead of this.
        for (k, v) in self.get("keybinds")?.iter() {
            let Some(k_input) = Input::parse(&k).ok() else {
                if k != "else" {
                    debug!("Config::get_keybind: failed to parse input: {:?}", k);
                }
                continue;
            };
            if k_input == input {
                return Some(v);
            }
        }
        None
    }

    pub fn get_syntax(&self) -> &HashMap<String, Vec<Regex>> {
        &self.current_config.syntax
    }

    pub fn get_syntax_sytle(&self) -> &HashMap<String, SyntaxStyle> {
        &self.current_config.syntax_style
    }

    pub fn get_editor(&self) -> &EditorConfig {
        &self.current_config.editor
    }

    pub fn get_theme(&self) -> &HashMap<String, Color> {
        &self.current_config.theme
    }

    pub fn get_theme_color(&self, name: &str) -> Option<Color> {
        self.current_config.theme.get(name).copied()
    }

    pub fn get_keybind_else(&self) -> Option<&[String]> {
        let else_value = self.get("keybinds")?.get("else")?;
        Some(&else_value)
    }

    fn rebuild_current_config(&mut self) {
        self.current_config = applied_config::build_applied_config(&self.modules, &self.state);
    }
}

#[derive(Debug)]
pub struct ConfigModule {
    // name: String,
    // path: PathBuf,
    mappings: Vec<ConditionalMapping>,
}

#[derive(Debug, Clone)]
struct ConditionalMapping {
    name: String,
    // All selectors must match for mapping to be active. Vacuous truth.
    selectors: Vec<Selector>,
    // Active mappings of the same layer merge together, but merged mappings on
    // higher layers replace those lower layers.
    layer: i32,
    entries: Vec<ConditionalMappingEntry>,
}

impl ConditionalMapping {
    pub fn is_active(&self, state: &ConfigState) -> bool {
        self.selectors.iter().all(|s| s.is_selected(state))
    }

    pub fn specificity(&self) -> usize {
        self.selectors.len()
    }
}

#[derive(Debug, Clone)]
struct ConditionalMappingEntry {
    pub name: TemplatedString,
    pub values: Vec<TemplatedString>,
}

#[derive(Debug, Clone)]
struct TemplatedString {
    parts: Vec<TemplatedStringPart>,
}

impl<'a> Into<TemplatedString> for &'a str {
    fn into(self) -> TemplatedString {
        TemplatedString {
            parts: vec![TemplatedStringPart::String(self.to_string())],
        }
    }
}

#[derive(Debug, Clone)]
enum TemplatedStringPart {
    String(String),
    Substitution(String),
}

#[derive(Debug, Clone)]
struct Selector {
    targeted_state: String,
    regex: Regex,
}

impl Selector {
    pub fn new(targeted_state: impl Into<String>, regex: &str) -> Result<Self, regex::Error> {
        let full_match_regex = format!("^{regex}$");
        let regex = regex::Regex::new(&full_match_regex)?;
        Ok(Self {
            targeted_state: targeted_state.into(),
            regex,
        })
    }

    pub fn is_selected(&self, state: &ConfigState) -> bool {
        let Some(target) = state.get(&self.targeted_state) else {
            return false;
        };
        self.regex.is_match_at(target, 0)
    }
}

#[derive(Debug, Default)]
pub struct ConfigState {
    states: HashMap<String, String>,
    // Ex:
    // "file" -> "src/lib.rs"
    // "mode" -> "text/edit"
    // "combo" -> ""
}

impl ConfigState {
    /// Active buffer's file path.
    pub const FILE: &'static str = "file";
    /// Active buffer's file format (language).
    pub const FORMAT: &'static str = "format";

    pub fn set(&mut self, state_name: impl Into<String>, value: impl Into<String>) {
        self.states.insert(state_name.into(), value.into());
    }

    pub fn get(&self, state_name: &str) -> Option<&str> {
        self.states.get(state_name).map(|s| s.as_str())
    }
}

pub fn make_builtin_config() -> Config {
    let mut conf = Config::default();
    macro_rules! builtin_cfg {
        ($s: expr) => {
            conf.add_module(include_str!($s)).unwrap()
        };
    }

    builtin_cfg!("./builtin/essentials.aycfg");
    builtin_cfg!("./builtin/formats/rust.aycfg");
    builtin_cfg!("./builtin/keybinds/base.aycfg");
    builtin_cfg!("./builtin/themes/base.aycfg");

    conf
}

fn parse_module(src: &str) -> Result<ConfigModule, ()> {
    use ayed_config_parser::ast;
    // TODO proper error handling

    let (ast, errors) = ayed_config_parser::parse_module(src);
    if !errors.is_empty() {
        debug!("{:?}", errors);
        return Err(());
    }

    fn aux(
        mappings: &mut Vec<ConditionalMapping>,
        mixins: &mut HashMap<String, Vec<ConditionalMapping>>,
        block: &ast::Block,
        selector_stack: &[Selector],
        parent_layer: i32,
        is_top_level: bool,
    ) {
        let layer = if block.is_override { 1 } else { parent_layer };
        match &block.kind {
            ast::BlockKind::Selector(ast::SelectorBlock {
                state_name,
                pattern,
                children,
            }) => {
                let mut selector_stack = selector_stack.to_vec();
                selector_stack.push(Selector::new(state_name.slice, pattern.slice).unwrap());

                for child in children {
                    aux(mappings, mixins, child, &selector_stack, layer, false);
                }
            }
            ast::BlockKind::Mapping(ast::MappingBlock {
                name,
                entries: ast_entries,
            }) => {
                let mut entries: Vec<ConditionalMappingEntry> = Vec::new();
                for entry in ast_entries {
                    let cond_entry = ConditionalMappingEntry {
                        name: entry.name.slice.into(),
                        values: entry
                            .values
                            .iter()
                            .map(|ast_template| {
                                let mut parts: Vec<TemplatedStringPart> = Vec::new();
                                let mut buf = String::new();
                                for ast_part in &ast_template.parts {
                                    buf = match ast_part {
                                        ast::TemplatePart::Span(span) => {
                                            buf.push_str(span.slice);
                                            buf
                                        }
                                        ast::TemplatePart::Escape(esc) => {
                                            esc.write(&mut buf);
                                            buf
                                        }
                                        ast::TemplatePart::Substitution(sub) => {
                                            parts.push(TemplatedStringPart::String(buf));
                                            parts.push(TemplatedStringPart::Substitution(
                                                sub.to_string(),
                                            ));
                                            String::new()
                                        }
                                    };
                                }
                                if !buf.is_empty() {
                                    parts.push(TemplatedStringPart::String(buf));
                                }

                                TemplatedString { parts }
                            })
                            .collect(),
                    };

                    entries.push(cond_entry);
                }
                mappings.push(ConditionalMapping {
                    name: name.to_string(),
                    selectors: selector_stack.to_vec(),
                    layer,
                    entries,
                });
            }
            ast::BlockKind::Mixin(ast::MixinBlock { name, children }) => {
                if !is_top_level {
                    unimplemented!("non top level mixins not supported yet");
                }

                let mut mixin_mappings = Vec::new();
                for child in children {
                    aux(
                        &mut mixin_mappings,
                        mixins,
                        child,
                        &selector_stack,
                        layer,
                        false,
                    );
                }
                mixins.insert(name.to_string(), mixin_mappings);
            }
            ast::BlockKind::Use(mixin_name) => {
                mappings.extend(mixins.get(mixin_name.slice).unwrap().iter().cloned().map(
                    |mut mapping| {
                        mapping.layer += parent_layer;
                        mapping.selectors.extend(selector_stack.iter().cloned());
                        mapping
                    },
                ));
            }
        }
    }

    let mut mappings = Vec::new();
    let mut mixins = HashMap::default();
    for block in &ast.top_level_blocks {
        aux(
            &mut mappings,
            &mut mixins,
            block,
            &[],
            Default::default(),
            true,
        );
    }
    Ok(ConfigModule { mappings })
}
