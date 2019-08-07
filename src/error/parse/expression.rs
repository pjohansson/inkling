//! Errors from parsing expressions in lines.

use std::{error::Error, fmt};

use crate::error::parse::LineError;

impl Error for ExpressionError {}

#[derive(Clone, Debug)]
/// Error from parsing `Expression` objects from strings.
pub struct ExpressionError {
    /// Content of string that could not parse into a valid `Expression`.
    pub content: String,
    /// Kind of error.
    pub kind: ExpressionErrorKind,
}

#[derive(Clone, Debug)]
pub enum ExpressionErrorKind {
    /// Empty expression string.
    Empty,
    /// The expression `head` was preceeded with an invalid operator ('*', '/', '%').
    InvalidHead { head: String },
    /// Could not parse variable inside expression.
    InvalidVariable(Box<LineError>),
    /// Encountered a string in the tail with no leading mathematical operator.
    NoOperator { content: String },
    /// Expression had unmatched parenthesis brackets.
    UnmatchedParenthesis,
}

impl fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ExpressionErrorKind::*;

        match &self.kind {
            Empty => write!(f, "cannot parse expression from empty string"),
            InvalidHead { head } => write!(
                f,
                "cannot parse expression from string '{}': no left hand side value before '{}'",
                self.content, head
            ),
            InvalidVariable(err) => write!(
                f,
                "cannot parse expression from string '{}': invalid variable: {}",
                self.content, err
            ),
            NoOperator { content } => write!(
                f,
                "cannot parse expression from string '{}': no mathematical operator before \
                 operand '{}'",
                self.content, content
            ),
            UnmatchedParenthesis => write!(
                f,
                "cannot parse expression from string '{}': found unmatched parenthesis",
                self.content,
            ),
        }
    }
}
