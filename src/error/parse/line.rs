//! Errors from parsing individual lines in stories.

use std::{error::Error, fmt};

use crate::{
    consts::{CHOICE_MARKER, STICKY_CHOICE_MARKER},
    error::{
        parse::{ConditionError, ExpressionError},
        utils::write_line_information,
    },
    utils::MetaData,
};

#[derive(Debug)]
/// Error from parsing individual lines in a story.
pub struct LineError {
    /// Line that caused the error.
    pub line: String,
    /// Kind of error.
    pub kind: LineErrorKind,
    /// Information about the origin of the line that caused this error.
    pub meta_data: MetaData,
}

#[derive(Debug)]
/// Variants of line errors.
pub enum LineErrorKind {
    /// Condition was invalid.
    ConditionError(ConditionError),
    /// Found a divert marker but no address.
    EmptyDivert,
    /// Found an empty expression (embraced part of line)
    EmptyExpression,
    /// Line did not end after a divert statement.
    ExpectedEndOfLine { tail: String },
    /// Could not read a numerical expression.
    ExpressionError(ExpressionError),
    /// Found several divert markers which indicates unimplemented tunnels.
    FoundTunnel,
    /// Found an address with invalid characters.
    InvalidAddress { address: String },
    /// A choice has both non-sticky and sticky markers.
    StickyAndNonSticky,
    /// Found unmatched curly braces.
    UnmatchedBraces,
    /// Found unmatched square brackets.
    UnmatchedBrackets,
}

impl Error for LineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

impl Error for LineErrorKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            LineErrorKind::ExpressionError(err) => Some(err),
            LineErrorKind::ConditionError(err) => Some(err),
            _ => None,
        }
    }
}

impl_from_error![
    LineErrorKind;
    [ConditionError, ConditionError],
    [ExpressionError, ExpressionError]
];

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_line_information(f, &self.meta_data)?;
        write!(f, "{}", self.kind)
    }
}

impl fmt::Display for LineErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LineErrorKind::*;

        match &self {
            ConditionError(err) => write!(f, "could not parse a condition: {}", err),
            EmptyDivert => write!(f, "encountered a divert statement with no address",),
            EmptyExpression => write!(f, "found an empty embraced expression ('{{}})'"),
            ExpectedEndOfLine { tail } => write!(
                f,
                "expected no more content after a divert statement address but found '{}'",
                tail
            ),
            ExpressionError(err) => write!(f, "could not parse an expression: {}", err),
            FoundTunnel => write!(
                f,
                "Found multiple divert markers. In the `Ink` language this indicates \
                 a `tunnel` for the story to pass through, but these are not yet implemented \
                 in `inkling`."
            ),
            InvalidAddress { address } => write!(
                f,
                "found an invalid address to knot, stitch or variable '{}': \
                 contains invalid characters",
                address
            ),
            StickyAndNonSticky => write!(
                f,
                "Encountered a line which has both non-sticky ('{}') and sticky ('{}') \
                 choice markers. This is not allowed.",
                CHOICE_MARKER, STICKY_CHOICE_MARKER
            ),
            UnmatchedBraces => write!(f, "line has unmatched curly '{{}}' braces"),
            UnmatchedBrackets => write!(f, "choice line has unmatched square '[]' brackets"),
        }
    }
}
