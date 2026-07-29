use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::range::Range;

#[derive(Default)]
pub struct Diagnostics {
    pub sources: HashMap<DiagnosticSource, HashMap<PathBuf, Vec<Diagnostic>>>,
}

impl Diagnostics {
    pub fn iter(&self) -> impl Iterator<Item = (&Path, &Diagnostic)> {
        self.sources
            .iter()
            .map(|(_, s)| s)
            .flat_map(|s| s.iter())
            .flat_map(|(k, v)| v.iter().map(|v2| (k.as_path(), v2)))
    }

    pub fn for_file(&self, path: &Path) -> impl Iterator<Item = &Diagnostic> {
        self.sources
            .iter()
            .map(|(_, s)| s)
            .flat_map(|s| s.get(path))
            .flat_map(|v| v)
    }

    pub fn stats(&self) -> DiagnosticStats {
        let mut stats = DiagnosticStats::default();
        let diags = self
            .sources
            .iter()
            .flat_map(|(_, s)| s.iter())
            .flat_map(|(_, diags)| diags.iter());
        for diag in diags {
            match diag.kind {
                DiagnosticKind::Error => stats.error_count += 1,
                DiagnosticKind::Warning => stats.warning_count += 1,
                _ => stats.other_count += 1,
            }
        }
        stats
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DiagnosticStats {
    pub error_count: i32,
    pub warning_count: i32,
    pub other_count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSource {
    Lsp,
}

pub enum DiagnosticKind {
    Error,
    Warning,
    Lint,
}

pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub range: Range,
    pub message: String,
}
