use std::fmt;

/// A single edit operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Start byte offset (inclusive)
    pub start_byte: usize,
    /// End byte offset (exclusive)
    pub end_byte: usize,
    /// Replacement content
    pub replacement: String,
    /// Optional description for logging
    pub description: Option<String>,
}

impl Edit {
    /// Create a new edit
    #[must_use]
    pub fn new(start_byte: usize, end_byte: usize, replacement: impl Into<String>) -> Self {
        Self {
            start_byte,
            end_byte,
            replacement: replacement.into(),
            description: None,
        }
    }

    /// Create an edit with description
    #[must_use]
    pub fn with_description(
        start_byte: usize,
        end_byte: usize,
        replacement: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            start_byte,
            end_byte,
            replacement: replacement.into(),
            description: Some(description.into()),
        }
    }

    /// Create a deletion edit
    #[must_use]
    pub fn delete(start_byte: usize, end_byte: usize) -> Self {
        Self::new(start_byte, end_byte, "")
    }

    /// Create an insertion edit (insert before position)
    #[must_use]
    pub fn insert(position: usize, content: impl Into<String>) -> Self {
        Self::new(position, position, content)
    }

    /// Length of the range being replaced
    #[must_use]
    pub const fn range_len(&self) -> usize {
        self.end_byte.saturating_sub(self.start_byte)
    }

    /// Check if this edit overlaps with another
    #[must_use]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start_byte < other.end_byte && other.start_byte < self.end_byte
    }
}

/// Error during rewriting
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteError {
    /// Two or more edits have overlapping ranges
    OverlappingEdits {
        /// Index of first overlapping edit
        edit_a: usize,
        /// Index of second overlapping edit
        edit_b: usize,
    },
    /// Edit range is out of bounds
    OutOfBounds {
        /// Index of the bad edit
        edit_index: usize,
        /// End byte of the edit
        end_byte: usize,
        /// Length of the source
        source_len: usize,
    },
    /// Source is not valid UTF-8 after edit
    InvalidUtf8,
    /// The rewritten source is not valid Python.
    InvalidPython {
        /// Parser error, including its source location.
        error: String,
    },
    /// Start offset exceeds the end offset.
    InvalidRange {
        /// Index of the invalid edit.
        edit_index: usize,
        /// Starting byte offset.
        start_byte: usize,
        /// Ending byte offset.
        end_byte: usize,
    },
}

impl fmt::Display for RewriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OverlappingEdits { edit_a, edit_b } => {
                write!(f, "Overlapping edits at indices {edit_a} and {edit_b}")
            }
            Self::OutOfBounds {
                edit_index,
                end_byte,
                source_len,
            } => {
                write!(
                    f,
                    "Edit {edit_index} out of bounds: end_byte {end_byte} > source length {source_len}"
                )
            }
            Self::InvalidUtf8 => write!(f, "Result is not valid UTF-8"),
            Self::InvalidPython { error } => write!(f, "Invalid Python after rewrite: {error}"),
            Self::InvalidRange {
                edit_index,
                start_byte,
                end_byte,
            } => {
                write!(
                    f,
                    "Edit {edit_index} has reversed range {start_byte}..{end_byte}"
                )
            }
        }
    }
}

impl std::error::Error for RewriteError {}

/// Builder for constructing multiple edits
#[derive(Debug, Default)]
pub struct EditBuilder {
    edits: Vec<Edit>,
}

impl EditBuilder {
    /// Create a new edit builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a replacement edit
    #[must_use]
    pub fn replace(
        mut self,
        start_byte: usize,
        end_byte: usize,
        replacement: impl Into<String>,
    ) -> Self {
        self.edits
            .push(Edit::new(start_byte, end_byte, replacement));
        self
    }

    /// Add a deletion edit
    #[must_use]
    pub fn delete(mut self, start_byte: usize, end_byte: usize) -> Self {
        self.edits.push(Edit::delete(start_byte, end_byte));
        self
    }

    /// Add an insertion edit
    #[must_use]
    pub fn insert(mut self, position: usize, content: impl Into<String>) -> Self {
        self.edits.push(Edit::insert(position, content));
        self
    }

    /// Build the list of edits
    #[must_use]
    pub fn build(self) -> Vec<Edit> {
        self.edits
    }
}
