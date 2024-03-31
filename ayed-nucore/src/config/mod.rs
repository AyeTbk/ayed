use std::collections::HashMap;

use regex::Regex;

use crate::input::Input;

pub mod commands;

macro_rules! map {
    () => {{
        HashMap::new()
    }};
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map = HashMap::new();
        $(
            map.insert($key, $val);
        )*
        map
    }};
}

#[derive(Default)]
pub struct Config {
    modules: Vec<ConfigModule>,
    state: ConfigState,
    current_config: AppliedConfig,
}

impl Config {
    pub fn get(&self, key: &str) -> Option<&HashMap<String, Vec<String>>> {
        self.current_config.get(key)
    }

    pub fn set_state(&mut self, state_name: impl Into<String>, value: impl Into<String>) {
        self.state.set(state_name, value);
        // TODO rebuild more efficiently instead of rebuilding completely
        self.rebuild_current_config();
    }

    pub fn get_keybind(&self, input: Input) -> Option<String> {
        self.get("keybinds")?
            .get(&input.to_string())
            .map(|v| v.join(" "))
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
                // current_mapping.extend(
                //     cond_mapping
                //         .mapping
                //         .iter()
                //         .map(|(k, v)| (k.to_string(), v.to_vec()));
                // )
            }
        }

        AppliedConfig { mappings }
    }
}

#[derive(Debug, Default)]
pub struct AppliedConfig {
    mappings: HashMap<String, HashMap<String, Vec<String>>>,
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
    let mut conf = Config {
        modules: vec![ConfigModule {
            mappings: vec![
                ConditionalMapping {
                    name: "keybinds".to_string(),
                    selectors: vec![],
                    mapping: map! {
                        "f".to_string() => vec!["edit".to_string(), "Cargo.toml".to_string()],
                        "else".to_string() => vec!["insert-char".to_string()],
                    },
                },
                ConditionalMapping {
                    name: "syntax-style".to_string(),
                    selectors: vec![],
                    mapping: map! {
                        "keyword".to_string() => vec!["#4488cf".to_string()],
                        "keyword-statement".to_string() => vec!["#aa77cc".to_string()],
                        "builtin".to_string() => vec!["#62b0fb".to_string(), "priority:11".to_string()],
                        "operator".to_string() => vec!["#ddccdd".to_string()],
                        "delimiter".to_string() => vec!["#ccaa11".to_string(), "priority:11".to_string()],
                        "macro".to_string() => vec!["#3377cc".to_string(), "priority:11".to_string()],
                        "type".to_string() => vec!["#55b89b".to_string(), "priority:12".to_string()],
                        "literal".to_string() => vec!["#aaddcc".to_string(), "priority:11".to_string()],
                        "string".to_string() => vec!["#bb8866".to_string(), "priority:14".to_string()],
                        "function".to_string() => vec!["#b8a4fc".to_string(), "priority:13".to_string()],
                        "namespace".to_string() => vec!["#55b89b".to_string()],
                        "comment".to_string() => vec!["#55887a".to_string(), "priority:15".to_string()],
                    },
                },
                ConditionalMapping {
                    name: "syntax".to_string(),
                    selectors: vec![Selector::new("file", r".*\.rs").unwrap()],
                    mapping: map! {
                        // All keywords
                        "keyword".to_string() => vec![r"\b(let|impl|pub|fn|mod|use|as|self|Self|mut|unsafe|move)\b".to_string() ,r"\b(struct|enum|type)\b".to_string()],
                        "keyword-statement".to_string() => vec![r"\b(if|else|while|for|in|loop|continue|break|match)\b".to_string()],
                        "builtin".to_string() => vec![r"\b(Some|None|Ok|Err)\b".to_string()],
                        "operator".to_string() => vec![r"(==|=|!=|\+|\+=|\-|\-=|\*|\*=|/|/=|!|\|\||&&|\||&|::|:|;|,|\.\.|\.|\?)".to_string()],
                        "delimiter".to_string() => vec![r"(->|=>|\{|\}|\[|\]|\(|\)|<|>)".to_string()],
                        "macro".to_string() => vec![r"\b([a-zA-Z0-9_]+\!)".to_string()],
                        "type".to_string() => vec![
                            r"\b([A-Z][a-zA-Z0-9_]*)\b".to_string(),
                            r"\b((u|i)(8|16|32|64|128)|f32|f64)\b".to_string(),
                            r"\b(char)\b".to_string(),
                        ],
                        "literal".to_string() => vec![
                            r"(([0-9]*\.[0-9]+|[0-9]+\.|[0-9]+)((u|i)(8|16|32|64|128)|f32|f64)?)".to_string(),
                            r"\b(true|false)\b".to_string(),
                        ],
                        "string".to_string() => vec![
                            "(r?\\\"[^\\\"]*\\\")".to_string(),
                            "(r?'[^']*')".to_string(),
                        ],
                        "function".to_string() => vec![r"\b([a-z0-9_][a-zA-Z0-9_]*)\(".to_string()],
                        "namespace".to_string() => vec![r"\b([a-zA-Z0-9_]+)::".to_string()],
                        "comment".to_string() => vec![r"(//.*)$".to_string()],
                    },
                },
                ConditionalMapping {
                    name: "hooks".into(),
                    selectors: vec![],
                    mapping: map! {
                        r"modify-buffer".to_string() => vec!["builtin-syntax-highlight".to_string()],
                        r"after-insert".to_string() => vec!["builtin-auto-indent".to_string()],
                    },
                },
            ],
        }],
        ..Default::default()
    };

    conf.rebuild_current_config();
    conf
}
