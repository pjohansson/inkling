//! Errors from parsing knots and stitches.

use std::{error::Error, fmt};

use crate::error::{
    parse::line::LineError,
    utils::{write_line_information, MetaData},
};

#[derive(Clone, Debug)]
/// Errors from parsing a single knot from lines.
pub struct KnotError {
    /// Information about the line at which the knot starts.
    pub knot_meta_data: MetaData,
    /// Set of errors that were encountered while parsing the knot.
    pub line_errors: Vec<KnotErrorKind>,
}

#[derive(Clone, Debug)]
/// Error from parsing a `Knot` or `Stitch` in a story.
pub enum KnotErrorKind {
    /// Duplicate knot name was found in a story.
    DuplicateKnotName {
        /// Name of duplicate stitch.
        name: String,
        /// Information about the origin of the line of the original knot with this name.
        prev_meta_data: MetaData,
    },
    /// Duplicate stitch name was found in a knot.
    DuplicateStitchName {
        /// Name of duplicate stitch.
        name: String,
        /// Name of knot that contains the stitches.
        knot_name: String,
        /// Information about the origin of the line that caused this error.
        meta_data: MetaData,
        /// Information about the origin of the line of the original stitch with this name.
        prev_meta_data: MetaData,
    },
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

impl Error for KnotError {}
impl Error for KnotNameError {}

impl Error for KnotErrorKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self {
            KnotErrorKind::InvalidName { kind, .. } => Some(kind),
            KnotErrorKind::LineError(err) => Some(err),
            _ => None,
        }
    }
}

impl_from_error![
    KnotErrorKind;
    [LineError, LineError]
];

/// Get a string with all errors from parsing a `Knot`.
pub(crate) fn write_knot_error<W: fmt::Write>(buffer: &mut W, error: &KnotError) -> fmt::Result {
    for line_error in &error.line_errors {
        match line_error {
            // All error kinds except these carry their own `MetaData` to use
            KnotErrorKind::EmptyKnot => {
                write_line_information(buffer, &error.knot_meta_data)?;
            }
            KnotErrorKind::DuplicateKnotName { .. } => {
                write_line_information(buffer, &error.knot_meta_data)?;
            }
            _ => (),
        }

        write!(buffer, "{}\n", line_error)?;
    }

    Ok(())
}

impl fmt::Display for KnotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} error(s) from parsing knot starting at line {}",
            self.line_errors.len(),
            self.knot_meta_data.line_index + 1,
        )
    }
}

impl fmt::Display for KnotErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use KnotErrorKind::*;

        match self {
            DuplicateKnotName {
                name,
                prev_meta_data,
            } => write!(
                f,
                "encountered another knot with name '{}' in the story (previous at {})",
                name, prev_meta_data
            ),
            DuplicateStitchName {
                name,
                knot_name,
                meta_data,
                prev_meta_data,
            } => {
                write_line_information(f, meta_data)?;
                write!(
                    f,
                    "encountered another stitch with name '{}' in knot '{}' (previous at {})",
                    name, knot_name, prev_meta_data
                )
            }
            EmptyKnot => write!(f, "knot has no content"),
            EmptyStitch {
                name: Some(name),
                meta_data,
            } => {
                write_line_information(f, meta_data)?;
                write!(f, "named stitch '{}' has no content", name)
            }
            EmptyStitch {
                name: None,
                meta_data,
            } => {
                write_line_information(f, meta_data)?;
                write!(f, "root stitch has no content",)
            }
            InvalidName {
                kind, meta_data, ..
            } => {
                write_line_information(f, meta_data)?;
                write!(f, "could not read knot or stitch name: {}", kind)
            }
            LineError(err) => write!(f, "{}", err),
        }
    }
}

impl fmt::Display for KnotNameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use KnotNameError::*;

        match self {
            ContainsWhitespace => write!(
                f,
                "name contains whitespace characters: only alphanumeric \
                 and underline characters are allowed"
            ),
            ContainsInvalidCharacter(c) => write!(
                f,
                "name contains invalid character '{}': only alphanumeric \
                 and underline characters are allowed",
                c
            ),
            Empty => write!(f, "no name after knot or stitch marker"),
            ReservedKeyword { ref keyword } => write!(
                f,
                "knot or stitch name may not be reserved keyword '{}'",
                keyword.to_lowercase()
            ),
        }
    }
}
