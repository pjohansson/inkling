//! Errors from parsing knots and stitches.

use std::{error::Error, fmt};

use crate::{error::parse::LineError, utils::MetaData};

impl Error for KnotErrorKind {}

#[derive(Debug)]
pub struct KnotError {
    pub knot_meta_data: MetaData,
    pub line_errors: Vec<KnotErrorKind>,
}

#[derive(Debug)]
/// Error from parsing a `Knot` or `Stitch` in a story.
pub enum KnotErrorKind {
    /// Knot has no content.
    EmptyKnot,
    /// Stitch in knot has no content.
    EmptyStitch {
        /// Name of stitch, if it is named.
        name: Option<String>,
        /// Information about the origin of the line that caused this error.
        meta_data: MetaData,
    },
    /// Could not parse a name for knot or stitch.
    InvalidName {
        /// String that could not be parsed into a name.
        line: String,
        /// Kind of error.
        kind: KnotNameError,
        /// Information about the origin of the line that caused this error.
        meta_data: MetaData,
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
        use KnotNameError::*;

        write!(f, "Could not parse a knot: ")?;

        match self {
            EmptyKnot => write!(f, "knot has no content"),
            EmptyStitch {
                name: Some(name),
                meta_data,
            } => write!(f, "named stitch '{}' has no content", name),
            EmptyStitch {
                name: None,
                meta_data,
            } => write!(f, "root stitch has no content"),
            InvalidName {
                line,
                kind,
                meta_data,
            } => {
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
                    Empty => {
                        write!(f, "no name after knot or stitch marker")?;
                    }
                    ReservedKeyword { ref keyword } => {
                        write!(
                            f,
                            "knot or stitch name may not be reserved keyword '{}'",
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
