//! Main error type from parsing and validating stories.
//!
use std::{error::Error, fmt};

use crate::error::parse::{InvalidAddressError, KnotErrorKind, LineError, PreludeError};

impl Error for ParseError {}

#[derive(Debug)]
/// Error from parsing text to construct a story.
pub enum ParseError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// Could not construct a `Knot` or `Stitch` as the content was read.
    KnotErrorKind(KnotErrorKind),
    /// Could not parse a individual line outside of knots.
    LineError(LineError),
    /// An invalid address was encountered when parsing the story.
    InvalidAddress(InvalidAddressError),
    /// Could not parse a line in the prelude.
    PreludeError(PreludeError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ParseError::*;

        match self {
            Empty => write!(f, "Tried to read from an empty file or string"),
            InvalidAddress(err) => write!(f, "{}", err),
            KnotErrorKind(err) => write!(f, "{}", err),
            LineError(err) => write!(f, "{}", err),
            PreludeError(err) => write!(f, "{}", err),
        }
    }
}

impl_from_error![
    ParseError;
    [InvalidAddress, InvalidAddressError],
    [KnotErrorKind, KnotErrorKind],
    [LineError, LineError],
    [PreludeError, PreludeError]
];
