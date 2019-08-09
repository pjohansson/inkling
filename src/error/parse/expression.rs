//! Errors from parsing expressions in lines.

use std::{error::Error, fmt};

use crate::error::parse::VariableError;

#[derive(Debug)]
/// Error from parsing `Expression` objects from strings.
pub struct ExpressionError {
    /// Content of string that could not parse into a valid `Expression`.
    pub content: String,
    /// Kind of error.
    pub kind: ExpressionErrorKind,
}

#[derive(Debug)]
/// Variant of `Expression` parsing error.
pub enum ExpressionErrorKind {
    /// Empty expression string.
    Empty,
    /// The expression `head` was preceeded with an invalid operator ('*', '/', '%').
    InvalidHead { head: String },
    /// Could not parse variable inside expression.
    InvalidVariable(VariableError),
    /// Encountered a string in the tail with no leading mathematical operator.
    NoOperator { content: String },
    /// Expression had unmatched parenthesis brackets.
    UnmatchedParenthesis,
}

impl Error for ExpressionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

impl Error for ExpressionErrorKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            ExpressionErrorKind::InvalidVariable(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} (expression string: '{}')", self.kind, self.content)
    }
}

impl fmt::Display for ExpressionErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ExpressionErrorKind::*;

        match &self {
            Empty => write!(f, "empty string"),
            InvalidHead { head } => write!(f, "no left hand side value before '{}'", head),
            InvalidVariable(err) => write!(f, "invalid variable: {}", err),
            NoOperator { content } => {
                write!(f, "no mathematical operator before operand '{}'", content)
            }
            UnmatchedParenthesis => write!(f, "found unmatched parenthesis",),
        }
    }
}
