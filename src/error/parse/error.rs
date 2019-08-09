//! Main error type from parsing and validating stories.

use std::{error::Error, fmt};

use crate::error::parse::{print_parse_error, InvalidAddressError, ParseError};

#[derive(Debug)]
/// Errors from reading a story.
///
/// A full print out of all individual errors can be made through
/// [`print_read_error`][crate::error::parse::print_read_error].
pub enum ReadError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// An invalid knot, stitch or divert address was encountered during validation.
    InvalidAddress(InvalidAddressError),
    /// Encountered one or more errors while parsing lines to construct the story.
    ParseError(ParseError),
}

/// Get a string containing all errors encountered while reading a story.
pub fn print_read_error(error: &ReadError) -> Result<String, fmt::Error> {
    match &error {
        ReadError::ParseError(parse_error) => print_parse_error(parse_error),
        _ => Ok(format!("{}", error)),
    }
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            ReadError::Empty => None,
            ReadError::InvalidAddress(err) => Some(err),
            ReadError::ParseError(err) => Some(err),
        }
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ReadError::*;

        match self {
            Empty => write!(f, "Could not parse story: no content was available"),
            InvalidAddress(err) => write!(f, "{}", err),
            ParseError(err) => write!(f, "{}", err),
        }
    }
}

impl_from_error![
    ReadError;
    [InvalidAddress, InvalidAddressError],
    [ParseError, ParseError]
];
