//! Data types for Python singleton pattern detection.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Category of detected Python singleton pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SingletonKind {
    /// Python `__new__` method overriding instance creation to return a cached instance.
    NewMethod,
    /// Python class with an instance attribute cache and `get_instance` / `getInstance` accessor.
    GetInstanceMethod,
    /// Python class decorated with `@singleton` or `@Singleton`.
    Decorator,
    /// Python class defined with `metaclass=Singleton`.
    Metaclass,
}

impl SingletonKind {
    /// Short human-readable identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NewMethod => "new_method",
            Self::GetInstanceMethod => "get_instance",
            Self::Decorator => "decorator",
            Self::Metaclass => "metaclass",
        }
    }

    /// Descriptive summary of the singleton anti-pattern variant.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::NewMethod => "Singleton via __new__ instance caching",
            Self::GetInstanceMethod => {
                "Singleton via instance attribute and get_instance() accessor"
            }
            Self::Decorator => "Singleton via class decorator",
            Self::Metaclass => "Singleton via metaclass",
        }
    }
}

impl std::fmt::Display for SingletonKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single detected singleton pattern in Python code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingletonMatch {
    /// Python source file containing the singleton pattern.
    pub file: PathBuf,
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number.
    pub column: usize,
    /// Name of the singleton class.
    pub class_name: String,
    /// Specific singleton pattern variant detected.
    pub kind: SingletonKind,
    /// Contextual snippet or class header.
    pub snippet: String,
}

/// Summary statistics of detected Python singletons.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingletonStats {
    /// Total detected singletons.
    pub total: usize,
    /// Count of `__new__` based singletons.
    pub new_methods: usize,
    /// Count of `get_instance` based singletons.
    pub get_instance_methods: usize,
    /// Count of decorator-based singletons.
    pub decorators: usize,
    /// Count of metaclass-based singletons.
    pub metaclasses: usize,
    /// Number of distinct files with at least one singleton.
    pub affected_files: usize,
}

impl SingletonStats {
    /// Records a match and increments counters.
    pub fn record(&mut self, kind: SingletonKind) {
        self.total += 1;
        match kind {
            SingletonKind::NewMethod => self.new_methods += 1,
            SingletonKind::GetInstanceMethod => self.get_instance_methods += 1,
            SingletonKind::Decorator => self.decorators += 1,
            SingletonKind::Metaclass => self.metaclasses += 1,
        }
    }
}

/// Overall results of a singleton pattern scan across Python files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingletonsResult {
    /// Aggregate counts.
    pub stats: SingletonStats,
    /// Number of Python files scanned.
    pub files_scanned: usize,
    /// All detected matches, sorted by file then line.
    pub matches: Vec<SingletonMatch>,
}

impl SingletonsResult {
    /// Returns `true` if no singletons were detected.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.stats.total == 0
    }
}
