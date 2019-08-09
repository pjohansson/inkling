//! Main error type from parsing and validating stories.
//!
use std::{error::Error, fmt};

use crate::error::parse::{InvalidAddressError, KnotError, PreludeError};

impl Error for ReadErrorKind {}

#[derive(Debug)]
/// Error from parsing text to construct a story.
pub enum ReadErrorKind {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// An invalid address was encountered when parsing the story.
    InvalidAddress(InvalidAddressError),
    /// Could not parse a line in the prelude.
    PreludeError(PreludeError),
    ParseError(ParseError),
}

impl fmt::Display for ReadErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ReadErrorKind::*;

        match self {
            Empty => write!(f, "Tried to read from an empty file or string"),
            InvalidAddress(err) => write!(f, "{}", err),
            PreludeError(err) => write!(f, "{}", err),
            ParseError(err) => unimplemented!(),
        }
    }
}

impl_from_error![
    ReadErrorKind;
    [InvalidAddress, InvalidAddressError],
    [PreludeError, PreludeError],
    [ParseError, ParseError]
];

#[derive(Debug)]
pub struct ParseError {
    pub knot_errors: Vec<KnotError>,
    pub prelude_errors: Vec<PreludeError>,
}
