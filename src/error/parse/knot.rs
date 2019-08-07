//! Errors from parsing knots and stitches.

use std::{error::Error, fmt};

use crate::error::parse::LineError;

impl Error for KnotErrorKind {}

#[derive(Debug)]
/// Error from parsing a `Knot` or `Stitch` in a story.
pub enum KnotErrorKind {
    /// Knot has no content.
    EmptyKnot,
    /// Stitch in knot has no content.
    EmptyStitch,
    /// Could not parse a name for knot or stitch.
    InvalidName {
        /// Line that was tried to parse into a name.
        line: String,
        /// Kind of error.
        kind: KnotNameError,
    },
    /// Could not parse a line inside a not.
    LineError(LineError),
}

#[derive(Clone, Debug)]
/// Invalid knot or stitch name.
pub enum KnotNameError {
    /// Knot name contains an invalid character.
    ContainsInvalidCharacter(char),
    /// Knot name contains a whitespace character.
    ContainsWhitespace,
    /// No name existed to read for the knot.
    Empty,
    /// No name existed to read for the knot.
    NoNamePresent,
    /// Name was a reserved keyword.
    ReservedKeyword { keyword: String },
}

impl_from_error![
    KnotErrorKind;
    [LineError, LineError]
];

impl fmt::Display for KnotErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use KnotErrorKind::*;
        use KnotNameError::Empty as EmptyKnotName;
        use KnotNameError::*;

        write!(f, "Could not parse a knot: ")?;

        match self {
            EmptyKnot => write!(f, "knot has no content"),
            EmptyStitch => write!(f, "stitch has not content"),
            InvalidName { line, kind } => {
                write!(f, "could not read knot name: ")?;

                match kind {
                    ContainsWhitespace => {
                        write!(
                            f,
                            "name contains whitespace characters: only alphanumeric \
                             and underline characters are allowed"
                        )?;
                    }
                    ContainsInvalidCharacter(c) => {
                        write!(
                            f,
                            "name contains invalid character '{}': only alphanumeric \
                             and underline characters are allowed",
                            c
                        )?;
                    }
                    EmptyKnotName => {
                        write!(f, "knot marker without a knot name was found")?;
                    }
                    NoNamePresent => {
                        write!(f, "knot or stitch has no name where one is expected")?;
                    }
                    ReservedKeyword { ref keyword } => {
                        write!(
                            f,
                            "Knot or stitch name may not be reserved keyword '{}'",
                            keyword.to_lowercase()
                        )?;
                    }
                }

                write!(f, " (line: {})", line)
            }
            LineError(err) => write!(f, "{}", err),
        }
    }
}
