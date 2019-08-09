//! Errors from parsing variables in lines.

use std::{error::Error, fmt};

#[derive(Debug)]
/// Error from parsing individual lines in a story.
pub struct VariableError {
    /// Content that caused the error.
    pub content: String,
    /// Kind of error.
    pub kind: VariableErrorKind,
}

#[derive(Debug)]
/// Variants of line errors.
pub enum VariableErrorKind {
    /// Found an address with invalid characters.
    InvalidAddress,
    /// Divert variable contained an invalid address.
    InvalidDivert { address: String },
    /// Number variable contained a number that could not be parsed.
    InvalidNumericValue { err: Box<dyn Error + 'static> },
}

impl Error for VariableError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

impl Error for VariableErrorKind {}

impl fmt::Display for VariableError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} in variable string '{}'", self.kind, self.content)
    }
}

impl fmt::Display for VariableErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use VariableErrorKind::*;

        match &self {
            InvalidAddress => write!(f, "invalid address"),
            InvalidDivert { address } => write!(f, "invalid divert address '{}'", address),
            InvalidNumericValue { .. } => write!(f, "could not parse number"),
        }
    }
}
