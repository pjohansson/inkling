//! Errors from parsing stories, knots, stitches and lines.

use std::{error::Error, fmt};

use crate::error::parse::LineParsingError;

impl Error for KnotError {}

#[derive(Debug)]
/// Error from parsing a `Knot` or `Stitch` in a story.
pub enum KnotError {
    /// Knot has no content.
    Empty,
    /// Stitch has no content.
    EmptyStitch,
    /// Could not parse a name for the knot. The offending string is encapsulated.
    InvalidName { line: String, kind: KnotNameError },
    /// Could not parse a line inside a not.
    LineError(LineParsingError),
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
    KnotError;
    [LineError, LineParsingError]
];

impl fmt::Display for KnotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use KnotError::Empty as EmptyKnot;
        use KnotError::*;
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
