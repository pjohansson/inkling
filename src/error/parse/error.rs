//! Main error type from parsing and validating stories.

use std::{error::Error, fmt};

use crate::error::parse::{
    address::InvalidAddressError,
    parse::{print_parse_error, ParseError},
    validate::print_invalid_address_errors,
};

#[derive(Debug)]
/// Errors from reading a story.
///
/// A full print out of all individual errors can be made through
/// [`print_read_error`][crate::error::parse::print_read_error].
pub enum ReadError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// One or more invalid knot, stitch or divert address was encountered during validation.
    InvalidAddress(Vec<InvalidAddressError>),
    /// Encountered one or more errors while parsing lines to construct the story.
    ParseError(ParseError),
}

/// Get a string containing all errors encountered while reading a story.
///
/// The errors are printed along with information about the line they were found in. Note that
/// this may not print *all* errors that were found. Line parsing stops after the first error
/// in every line, so lines containing more than one error will only have the first show up
/// in this list.
///
/// Furthermore, since parsing and validation is done separately, this function will only
/// print errors found in either step, not both. A file that could not be parsed may have
/// additional problems that will be discovered during the validation step.
pub fn print_read_error(error: &ReadError) -> Result<String, fmt::Error> {
    match &error {
        ReadError::InvalidAddress(errors) => print_invalid_address_errors(errors),
        ReadError::ParseError(parse_error) => print_parse_error(parse_error),
        _ => Ok(format!("{}", error)),
    }
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            ReadError::ParseError(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ReadError::*;

        match self {
            Empty => write!(f, "Could not parse story: no content was available"),
            InvalidAddress(err) => write!(
                f,
                "Could not validate story: found {} invalid addresses",
                err.len()
            ),
            ParseError(err) => write!(f, "{}", err),
        }
    }
}

impl_from_error![
    ReadError;
    [ParseError, ParseError]
];

impl From<Vec<InvalidAddressError>> for ReadError {
    fn from(errors: Vec<InvalidAddressError>) -> Self {
        ReadError::InvalidAddress(errors)
    }
}
