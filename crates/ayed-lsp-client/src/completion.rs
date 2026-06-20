use std::collections::HashSet;

use crate::types::CompletionItem;

#[derive(Default)]
pub struct Completion {
    items: Vec<CompletionItem>,
    generation: u32,
    resolving: HashSet<usize>,
}

impl Completion {
    pub fn items(&self) -> &[CompletionItem] {
        &self.items
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn set_items(&mut self, items: Vec<CompletionItem>) {
        self.items = items;
        self.generation = self.generation.wrapping_add(1);
        self.resolving = HashSet::new();
    }

    pub fn resolve_item(&mut self, idx: u32, generation: u32, item: CompletionItem) -> bool {
        if generation != self.generation {
            log::warn!("ignored a stale completion item resolve.");
            return false;
        }
        let idx = idx as usize;
        self.items[idx] = item;
        self.resolving.remove(&idx);
        return true;
    }

    pub fn all_items_are_resolved(&self) -> bool {
        self.resolving.is_empty()
    }
}
