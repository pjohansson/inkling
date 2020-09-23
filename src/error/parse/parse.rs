//! Main error type from parsing lines into a story.

use std::{
    error::Error,
    fmt::{self, Write},
};

use crate::error::parse::{
    knot::{write_knot_error, KnotError},
    prelude::PreludeError,
};

impl Error for ParseError {}

#[derive(Clone, Debug)]
/// List of errors encountered when parsing a story.
///
/// Note that this may not contain all errors in the story. Individual lines return an error
/// as soon as they encounter one, which means that they may contain additional errors beyond
/// their first.
pub struct ParseError {
    /// Errors from lines in the prelude.
    pub prelude_errors: Vec<PreludeError>,
    /// Errors from lines in knots.
    ///
    /// Each element in this list corresponds to a separate knot in the story.
    pub knot_errors: Vec<KnotError>,
}

/// Get a string containing all line errors encountered when parsing a story.
pub(crate) fn print_parse_error(error: &ParseError) -> Result<String, fmt::Error> {
    let mut buffer = String::new();

    for prelude_error in &error.prelude_errors {
        write!(&mut buffer, "{}\n", prelude_error)?;
    }

    for knot_error in &error.knot_errors {
        write_knot_error(&mut buffer, knot_error)?;
    }

    Ok(buffer)
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let num_errors = self.prelude_errors.len()
            + self
                .knot_errors
                .iter()
                .map(|error| error.line_errors.len())
                .sum::<usize>();

        write!(
            f,
            "Could not parse story: found {} errors in lines.",
            num_errors
        )
    }
}
