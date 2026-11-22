//! A single rule violation and a report helper.

use std::fmt;

/// One broken rule, located by file (or crate) name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub file: String,
    pub detail: String,
}

impl Violation {
    #[must_use]
    pub fn new(file: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { file: file.into(), detail: detail.into() }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.detail)
    }
}

/// One violation per line, for assertion messages.
#[must_use]
pub fn report(violations: &[Violation]) -> String {
    violations.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
}
