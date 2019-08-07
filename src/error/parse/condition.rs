//! Errors from parsing conditions in lines.

use std::{error::Error, fmt};

use crate::error::parse::VariableError;

impl Error for ConditionError {}

#[derive(Debug)]
/// Error from parsing `Condition` objects.
pub struct ConditionError {
    /// Content of string that caused the error.
    content: String,
    /// Error variant.
    kind: ConditionErrorKind,
}

#[derive(Debug)]
/// Variant of `Condition` parsing error.
pub enum ConditionErrorKind {
    /// The first item in a condition was not `Blank` or any other item was not `And` or `Or`.
    ///
    /// This is an internal consistency check from parsing a condition. Every subsequent
    /// condition to the first should be preceeded by an `and` or `or` marker, while the
    /// first condition should not be. After parsing the condition we assert that this is true.
    /// If not, some internal shenanigans are going on, but this should be unreachable.
    BadLink,
    /// Could not parse a number from the condition.
    BadValue,
    /// Generic error.
    CouldNotParse,
    /// Could not parse a variable.
    CouldNotParseVariable(Box<VariableError>),
    /// The line had multiple else statements.
    MultipleElseStatements,
    /// There was no condition in the line.
    NoCondition,
    /// Found unmatched parenthesis.
    UnmatchedParenthesis,
}

impl fmt::Display for ConditionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConditionErrorKind::*;

        match &self.kind {
            BadLink => write!(
                f,
                "internal error: did not correctly partition conditions into parts separated \
                 by `and`/`or` markers"
            ),
            BadValue => write!(f, "could not parse a number from the condition value"),
            CouldNotParse => write!(f, "incorrectly formatted condition"),
            CouldNotParseVariable(err) => {
                write!(f, "could not parse variable in condition: {}", err)
            }
            MultipleElseStatements => write!(f, "found multiple else statements in condition"),
            NoCondition => write!(f, "condition string was empty"),
            UnmatchedParenthesis => write!(f, "contained unmatched parenthesis"),
        }?;

        write!(f, " (condition string: '{}')", &self.content)
    }
}

impl ConditionError {
    /// Quickly construct an error from the kind and line.
    pub fn from_kind<T: Into<String>>(content: T, kind: ConditionErrorKind) -> Self {
        ConditionError {
            content: content.into(),
            kind,
        }
    }
}
