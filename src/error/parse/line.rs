//! Errors from parsing individual lines in stories.

use std::{error::Error, fmt};

use crate::{
    consts::{CHOICE_MARKER, STICKY_CHOICE_MARKER},
    error::parse::{ConditionError, ExpressionError},
};

impl Error for LineError {}

#[derive(Clone, Debug)]
/// Error from parsing individual lines in a story.
pub struct LineError {
    /// Line that caused the error.
    pub line: String,
    /// Kind of error.
    pub kind: LineErrorKind,
}

#[derive(Clone, Debug)]
/// Variants of line errors.
pub enum LineErrorKind {
    /// Condition was invalid.
    ConditionError(ConditionError),
    /// Could not read a numerical expression.
    BadExpression(ExpressionError),
    /// Found a divert marker but no address.
    EmptyDivert,
    /// Found an empty expression (embraced part of line)
    EmptyExpression,
    /// Line did not end after a divert statement.
    ExpectedEndOfLine { tail: String },
    /// Found several divert markers which indicates unimplemented tunnels.
    FoundTunnel,
    /// Found an address with invalid characters.
    InvalidAddress { address: String },
    /// Could not parse a variable.
    InvalidVariable { content: String },
    /// Divert variable contained an invalid address.
    InvalidVariableDivert { address: String, content: String },
    /// Number variable contained a number that could not be parsed.
    InvalidVariableNumber { content: String },
    /// No variable name after a VAR statement.
    NoVariableName,
    /// A choice has both non-sticky and sticky markers.
    StickyAndNonSticky,
    /// Found unmatched curly braces.
    UnmatchedBraces,
    /// Found unmatched square brackets.
    UnmatchedBrackets,
}

impl LineError {
    /// Constructor of error from some string and kind.
    pub fn from_kind<T: Into<String>>(line: T, kind: LineErrorKind) -> Self {
        LineError {
            line: line.into(),
            kind,
        }
    }
}

impl_from_error![
    LineErrorKind;
    [ConditionError, ConditionError],
    [BadExpression, ExpressionError]
];

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LineErrorKind::*;

        match &self.kind {
            ConditionError(err) => write!(f, "Could not parse a condition: {}", err),
            BadExpression(err) => write!(f, "Could not parse an expression: {}", err),
            EmptyDivert => write!(f, "Encountered a divert statement with no address",),
            EmptyExpression => write!(f, "Found an empty embraced expression ({{}})"),
            ExpectedEndOfLine { tail } => write!(
                f,
                "Expected no more content after a divert statement address but found '{}'",
                tail
            ),
            FoundTunnel => write!(
                f,
                "Found multiple divert markers in a line. In the `Ink` language this indicates \
                 a `tunnel` for the story to pass through, but these are not yet implemented \
                 in `inkling`."
            ),
            InvalidAddress { address } => write!(
                f,
                "Found an invalid address to knot, stitch or variable '{}': \
                 contains invalid characters",
                address
            ),
            InvalidVariable { content } => {
                write!(f, "Could not parse a variable from '{}'", content)
            }
            InvalidVariableDivert { address, content } => write!(
                f,
                "Invalid divert address '{}' when parsing variable from '{}'",
                address, content
            ),
            InvalidVariableNumber { content } => {
                write!(f, "Invalid number '{}' when parsing variable", content)
            }
            NoVariableName => write!(f, "No variable name for variable assignment"),
            StickyAndNonSticky => write!(
                f,
                "Encountered a line which has both non-sticky ('{}') and sticky ('{}') \
                 choice markers. This is not allowed.",
                CHOICE_MARKER, STICKY_CHOICE_MARKER
            ),
            UnmatchedBraces => write!(f, "Line has unmatched curly '{{}}' braces"),
            UnmatchedBrackets => write!(f, "Choice line has unmatched square '[]' brackets"),
        }?;

        write!(f, " (line: {}", &self.line)
    }
}
