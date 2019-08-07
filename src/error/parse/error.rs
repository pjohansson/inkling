//! Main error type from parsing and validating stories.
//!
use std::{error::Error, fmt};

use crate::error::parse::{InvalidAddressError, KnotError, LineError};

impl Error for ParseError {}

#[derive(Debug)]
/// Error from parsing text to construct a story.
pub enum ParseError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// Could not construct a `Knot` or `Stitch` as the content was read.
    KnotError(KnotError),
    /// Could not parse a individual line outside of knots.
    LineError(LineError),
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
    [LineError, LineError]
];
