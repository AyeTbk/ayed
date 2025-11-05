use std::collections::{BTreeMap, HashMap};

use regex::Regex;

use crate::{
    config::{ConditionalMapping, ConfigModule, ConfigState},
    ui::Color,
};

#[derive(Debug, Default)]
pub struct AppliedConfig {
    pub mappings: HashMap<String, HashMap<String, Vec<String>>>,
    pub syntax: HashMap<String, Vec<Regex>>,
    pub editor: EditorConfig,
    pub theme: HashMap<String, Color>,
}

impl AppliedConfig {
    pub fn get(&self, key: &str) -> Option<&HashMap<String, Vec<String>>> {
        self.mappings.get(key)
    }
}

pub fn build_applied_config(modules: &Vec<ConfigModule>, state: &ConfigState) -> AppliedConfig {
    let mut active_mappings: HashMap<&str, Vec<&ConditionalMapping>> = Default::default();

    // Gather all active mappings
    for module in modules {
        for cond_mapping in &module.mappings {
            if !cond_mapping.is_active(state) {
                continue;
            }

            active_mappings
                .entry(&cond_mapping.name)
                .or_default()
                .push(&cond_mapping);
        }
    }

    #[derive(Default)]
    struct Mapping {
        entries: HashMap<String, Vec<String>>,
        specificity: usize,
    }

    enum MergeStrategy {
        MergeMoreSpecificFirst,
        ReplaceWithMoreSpecific,
    }

    // Merge active mappings, giving priority to the ones with more specific selectors.
    let mut layers_of_mappings: BTreeMap<i32, HashMap<String, Mapping>> = Default::default();

    for (mapping_name, mut active_mappings) in active_mappings {
        active_mappings.sort_by_key(|a| a.specificity());
        for active_mapping in active_mappings {
            let mappings = layers_of_mappings.entry(active_mapping.layer).or_default();

            let Some(mapping) = mappings.get_mut(mapping_name) else {
                mappings.insert(
                    mapping_name.to_string(),
                    Mapping {
                        entries: active_mapping.entries.clone(),
                        specificity: active_mapping.specificity(),
                    },
                );
                continue;
            };

            for (key, values) in &active_mapping.entries {
                let Some(existing_values) = mapping.entries.get_mut(key) else {
                    mapping.entries.insert(key.to_string(), values.to_vec());
                    continue;
                };

                let has_same_specificity_as_last =
                    mapping.specificity == active_mapping.specificity();
                // TODO The "hooks" mapping is special for now, but a way
                // to control the strategy of specific mappings should
                // probably be exposed in some way in the config.
                let use_merge_strat = mapping_name == "hooks" || has_same_specificity_as_last;
                let strategy = if use_merge_strat {
                    MergeStrategy::MergeMoreSpecificFirst
                } else {
                    MergeStrategy::ReplaceWithMoreSpecific
                };

                let mut more_specific_values = values.to_vec();
                match strategy {
                    MergeStrategy::MergeMoreSpecificFirst => {
                        std::mem::swap(existing_values, &mut more_specific_values);
                        existing_values.extend(more_specific_values);
                    }
                    MergeStrategy::ReplaceWithMoreSpecific => {
                        *existing_values = more_specific_values;
                    }
                }
            }
        }
    }

    // Merge mappings, respecting layers
    let mut mappings: HashMap<String, HashMap<String, Vec<String>>> = Default::default();
    for (_, layer_mappings) in layers_of_mappings.into_iter() {
        let layer_mappings_iter = layer_mappings.into_iter().map(|l| (l.0, l.1.entries));
        mappings.extend(layer_mappings_iter);
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

    let mut editor = EditorConfig::default();
    if let Some(mapping) = mappings.get("editor") {
        if let Some(indent_size) = mapping
            .get("indent-size")
            .and_then(|v| v.first())
            .and_then(|s| u8::from_str_radix(s, 10).ok())
        {
            editor.indent_size = indent_size as i32;
        }
    }

    let theme = mappings
        .get("theme")
        .unwrap_or(&HashMap::new())
        .iter()
        .flat_map(|(rule_name, values)| {
            let color = Color::from_hex(values.first()?).ok()?;
            Some((rule_name.to_string(), color))
        })
        .collect();

    AppliedConfig {
        mappings,
        syntax,
        editor,
        theme,
    }
}

#[derive(Debug)]
pub struct EditorConfig {
    pub indent_size: i32,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { indent_size: 4 }
    }
}
