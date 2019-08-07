//! Errors from parsing variables in lines.

use std::{error::Error, fmt};

impl Error for VariableError {}

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
    InvalidNumericValue { err: Box<dyn Error> },
}

impl fmt::Display for VariableError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use VariableErrorKind::*;

        match &self.kind {
            InvalidAddress => write!(f, "invalid address in variable string '{}'", self.content),
            InvalidDivert { address } => write!(
                f,
                "invalid divert address '{}' in variable string '{}'",
                address, self.content
            ),
            InvalidNumericValue { .. } => {
                write!(f, "could not parse number from '{}'", self.content)
            }
        }
    }
}
