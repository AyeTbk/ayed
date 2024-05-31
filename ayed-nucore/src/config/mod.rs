use std::collections::HashMap;

use regex::Regex;

use crate::input::Input;

pub mod commands;

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

    pub fn get(&self, key: &str) -> Option<&HashMap<String, Vec<String>>> {
        self.current_config.get(key)
    }

    pub fn set_state(&mut self, state_name: impl Into<String>, value: impl Into<String>) {
        self.state.set(state_name, value);
        // TODO rebuild more efficiently instead of rebuilding completely
        self.rebuild_current_config();
    }

    pub fn get_keybind(&self, input: Input) -> Option<String> {
        // TODO have a map of actual inputs in the Applied config instead of this.
        for (k, v) in self.get("keybinds")? {
            let Some(k_input) = Input::parse(&k).ok() else {
                if k != "else" {
                    eprintln!("Config::get_keybind: failed to parse input: {:?}", k);
                }
                continue;
            };
            if k_input == input {
                return Some(v.join(" "));
            }
        }
        None
    }

    pub fn get_syntax(&self) -> &HashMap<String, Vec<Regex>> {
        &self.current_config.syntax
    }

    pub fn get_keybind_else_insert_char(&self) -> bool {
        (|| Some(self.get("keybinds")?.get("else")?.get(0)? == "insert-char"))().unwrap_or(false)
    }

    fn rebuild_current_config(&mut self) {
        self.current_config = Self::build_applied_config(&self.modules, &self.state);
    }

    fn build_applied_config(modules: &Vec<ConfigModule>, state: &ConfigState) -> AppliedConfig {
        let mut active_mappings: HashMap<String, Vec<&ConditionalMapping>> = Default::default();

        // Gather all active mappings
        for module in modules {
            for cond_mapping in &module.mappings {
                if !cond_mapping.is_active(state) {
                    continue;
                }

                active_mappings
                    .entry(cond_mapping.name.clone()) // FIXME Unecessary allocation
                    .or_default()
                    .push(&cond_mapping);
            }
        }

        // Merge active mappings, giving priority to the ones with more specific selectors
        let mut mappings: HashMap<String, HashMap<String, Vec<String>>> = Default::default();
        for (mapping_name, mut cond_mappings) in active_mappings {
            cond_mappings.sort_by(|a, b| a.selector_specificity_cmp(b));
            let current_mapping = mappings.entry(mapping_name.clone()).or_default(); // FIXME Unecessary allocation
            for cond_mapping in cond_mappings {
                for (key, values) in &cond_mapping.mapping {
                    let existing_values = current_mapping
                        .entry(key.to_string()) // FIXME Unecessary allocations
                        .or_default();
                    let mut more_specific_values = values.to_vec();

                    // Make sure to merge list of values putting more specific values first
                    std::mem::swap(existing_values, &mut more_specific_values);
                    existing_values.extend(more_specific_values);
                }

                // TODO remember that this commented code was for and why it got commented out
                // NOTE couple months later: i confirm i dont remember
                // current_mapping.extend(
                //     cond_mapping
                //         .mapping
                //         .iter()
                //         .map(|(k, v)| (k.to_string(), v.to_vec()));
                // )
            }
        }

        let syntax = mappings
            .get("syntax")
            .unwrap_or(&HashMap::new())
            .iter()
            .map(|(rule_name, patterns)| {
                let regexes = patterns
                    .iter()
                    .map(|pat| Regex::new(pat).unwrap())
                    .collect();
                (rule_name.to_string(), regexes)
            })
            .collect();

        AppliedConfig { mappings, syntax }
    }
}

#[derive(Debug, Default)]
pub struct AppliedConfig {
    mappings: HashMap<String, HashMap<String, Vec<String>>>,
    syntax: HashMap<String, Vec<Regex>>,
}

impl AppliedConfig {
    fn get(&self, key: &str) -> Option<&HashMap<String, Vec<String>>> {
        self.mappings.get(key)
    }
}

pub struct ConfigModule {
    // name: String,
    // path: PathBuf,
    mappings: Vec<ConditionalMapping>,
}

struct ConditionalMapping {
    name: String,
    // All selectors must match for mapping to be active. Vacuous truth.
    selectors: Vec<Selector>,
    mapping: HashMap<String, Vec<String>>,
}

impl ConditionalMapping {
    pub fn is_active(&self, state: &ConfigState) -> bool {
        self.selectors.iter().all(|s| s.is_selected(state))
    }

    pub fn selector_specificity_cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.selectors.len().cmp(&other.selectors.len())
    }
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
    pub const FILE: &'static str = "file";

    pub fn set(&mut self, state_name: impl Into<String>, value: impl Into<String>) {
        self.states.insert(state_name.into(), value.into());
    }

    pub fn get(&self, state_name: &str) -> Option<&str> {
        self.states.get(state_name).map(|s| s.as_str())
    }
}

pub fn make_builtin_config() -> Config {
    let mut conf = Config::default();
    conf.add_module(include_str!("./builtin.ayedconf")).unwrap();
    conf
}

fn parse_module(src: &str) -> Result<ConfigModule, ()> {
    use ayed_config_parser::ast;
    // TODO proper error handling

    let (ast, errors) = ayed_config_parser::parse_module(src);
    if !errors.is_empty() {
        dbg!(errors);
        return Err(());
    }

    fn aux(
        mappings: &mut Vec<ConditionalMapping>,
        block: &ast::Block,
        selector_stack: &[Selector],
    ) {
        match block {
            ast::Block::SelectorBlock(ast::SelectorBlock {
                state_name,
                pattern,
                children,
            }) => {
                let mut selector_stack = selector_stack.to_vec();
                selector_stack.push(Selector::new(state_name.slice, pattern.slice).unwrap());

                for child in children {
                    aux(mappings, child, &selector_stack);
                }
            }
            ast::Block::MappingBlock(ast::MappingBlock { name, entries }) => {
                let mapping = entries
                    .iter()
                    .map(|entry| {
                        (
                            entry.name.to_string(),
                            entry.value.slice.split(' ').map(str::to_string).collect(),
                        )
                    })
                    .collect();
                mappings.push(ConditionalMapping {
                    name: name.to_string(),
                    selectors: selector_stack.to_vec(),
                    mapping,
                });
            }
        }
    }

    let mut mappings = Vec::new();
    for block in &ast.top_level_blocks {
        aux(&mut mappings, block, &[])
    }
    Ok(ConfigModule { mappings })
}
