use std::{error::Error, fmt};

pub(crate) mod condition;
pub(crate) mod expression;
pub(crate) mod knot;
pub(crate) mod line;
pub(crate) mod validate;

pub(crate) use condition::{BadCondition, BadConditionKind};
pub(crate) use expression::{ExpressionError, ExpressionErrorKind};
pub(crate) use knot::{KnotError, KnotNameError};
pub(crate) use line::{LineErrorKind, LineParsingError};
pub(crate) use validate::InvalidAddressError;

impl Error for ParseError {}

#[derive(Debug)]
/// Error from parsing text to construct a story.
pub enum ParseError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// Could not construct a `Knot` or `Stitch` as the content was read.
    KnotError(KnotError),
    /// Could not parse a individual line outside of knots.
    LineError(LineParsingError),
    /// An invalid address was encountered when parsing the story.
    InvalidAddress(InvalidAddressError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ParseError::*;

        match self {
            Empty => write!(f, "Tried to read from an empty file or string"),
            InvalidAddress(err) => write!(f, "{}", err),
            KnotError(err) => write!(f, "{}", err),
            LineError(err) => write!(f, "{}", err),
        }
    }
}

impl_from_error![
    ParseError;
    [InvalidAddress, InvalidAddressError],
    [KnotError, KnotError],
    [LineError, LineParsingError]
];
