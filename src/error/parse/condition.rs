//! Errors from parsing conditions in lines.

use std::{error::Error, fmt};

use crate::error::parse::{expression::ExpressionError, variable::VariableError};

#[derive(Debug)]
/// Error from parsing `Condition` objects.
pub struct ConditionError {
    /// Content of string that caused the error.
    pub content: String,
    /// Error variant.
    pub kind: ConditionErrorKind,
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
    /// Could not parse an expression for either side of a `lhs (cmp) rhs` condition.
    InvalidExpression(ExpressionError),
    /// Could not parse a variable.
    InvalidVariable(VariableError),
    /// The line had multiple else statements.
    MultipleElseStatements,
    /// There was no condition in the line.
    NoCondition,
    /// Found unmatched parenthesis.
    UnmatchedParenthesis,
}

impl Error for ConditionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

impl Error for ConditionErrorKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            ConditionErrorKind::InvalidVariable(err) => Some(err),
            _ => None,
        }
    }
}

impl_from_error![
    ConditionErrorKind;
    [InvalidExpression, ExpressionError],
    [InvalidVariable, VariableError]
];

impl fmt::Display for ConditionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} (condition string: '{}')", &self.kind, &self.content)
    }
}

impl fmt::Display for ConditionErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ConditionErrorKind::*;

        match &self {
            BadLink => write!(
                f,
                "internal error: did not correctly partition conditions into parts separated \
                 by `and`/`or` markers"
            ),
            BadValue => write!(f, "could not parse a number from the condition value"),
            CouldNotParse => write!(f, "incorrectly formatted condition"),
            InvalidExpression(err) => write!(
                f,
                "could not parse left or right hand side expression for a comparison: {}",
                err
            ),
            InvalidVariable(err) => write!(f, "could not parse variable in condition: {}", err),
            MultipleElseStatements => write!(f, "found multiple else statements in condition"),
            NoCondition => write!(f, "condition string was empty"),
            UnmatchedParenthesis => write!(f, "contained unmatched parenthesis"),
        }
    }
}

impl ConditionError {
    /// Quickly construct an error from the kind and line.
    pub(crate) fn from_kind<T: Into<String>>(content: T, kind: ConditionErrorKind) -> Self {
        ConditionError {
            content: content.into(),
            kind,
        }
    }
}
