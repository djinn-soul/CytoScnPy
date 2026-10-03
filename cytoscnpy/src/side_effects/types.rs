//! Data types for module-level import-time side-effect detection.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Category of detected module-level side effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectKind {
    /// Python top-level function or method invocation executed on module import.
    PythonTopLevelCall,
    /// Python top-level loop (`for` or `while`) executed on module import.
    PythonTopLevelLoop,
    /// Python top-level `with` statement executed on module import.
    PythonTopLevelWith,
    /// `JavaScript` / `TypeScript` top-level framework, server, or network call.
    JsTopLevelCall,
    /// `JavaScript` / `TypeScript` global event listener attached at module load.
    JsEventListener,
}

impl SideEffectKind {
    /// Short human-readable identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PythonTopLevelCall => "python_toplevel_call",
            Self::PythonTopLevelLoop => "python_toplevel_loop",
            Self::PythonTopLevelWith => "python_toplevel_with",
            Self::JsTopLevelCall => "js_toplevel_call",
            Self::JsEventListener => "js_event_listener",
        }
    }

    /// Descriptive summary of the anti-pattern.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::PythonTopLevelCall => "Module-level function call executed at import time",
            Self::PythonTopLevelLoop => "Module-level loop executed at import time",
            Self::PythonTopLevelWith => "Module-level with-block executed at import time",
            Self::JsTopLevelCall => "Top-level framework or network call executed on script load",
            Self::JsEventListener => "Top-level event listener registered on script load",
        }
    }
}

impl std::fmt::Display for SideEffectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single detected module-level side effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffectMatch {
    /// Source file containing the side effect.
    pub file: PathBuf,
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number.
    pub column: usize,
    /// Type of side effect detected.
    pub kind: SideEffectKind,
    /// Contextual snippet or statement text.
    pub snippet: String,
}

/// Summary counts of detected side effects.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffectStats {
    /// Total detected side effects.
    pub total: usize,
    /// Count of Python top-level function calls.
    pub python_calls: usize,
    /// Count of Python top-level loops.
    pub python_loops: usize,
    /// Count of Python top-level with statements.
    pub python_withs: usize,
    /// Count of `JavaScript` / `TypeScript` top-level calls.
    pub js_calls: usize,
    /// Count of `JavaScript` / `TypeScript` global event listeners.
    pub js_event_listeners: usize,
    /// Number of distinct files with at least one side effect.
    pub affected_files: usize,
}

impl SideEffectStats {
    /// Records a match and increments the corresponding counter.
    pub fn record(&mut self, kind: SideEffectKind) {
        self.total += 1;
        match kind {
            SideEffectKind::PythonTopLevelCall => self.python_calls += 1,
            SideEffectKind::PythonTopLevelLoop => self.python_loops += 1,
            SideEffectKind::PythonTopLevelWith => self.python_withs += 1,
            SideEffectKind::JsTopLevelCall => self.js_calls += 1,
            SideEffectKind::JsEventListener => self.js_event_listeners += 1,
        }
    }
}

/// Overall results of a module-level side-effects scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffectsResult {
    /// Aggregate statistics.
    pub stats: SideEffectStats,
    /// Total number of files scanned.
    pub files_scanned: usize,
    /// All detected matches sorted by file then line.
    pub matches: Vec<SideEffectMatch>,
}

impl SideEffectsResult {
    /// Returns `true` if no side effects were detected.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.stats.total == 0
    }
}
