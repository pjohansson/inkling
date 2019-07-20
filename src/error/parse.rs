use std::{error::Error, fmt};

use crate::consts::{CHOICE_MARKER, KNOT_MARKER, STICKY_CHOICE_MARKER};

#[derive(Debug)]
/// Error from parsing text to construct a story.
pub enum ParseError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// Error from constructing a knot.
    KnotError(KnotError),
    /// Error from parsing a single line.
    LineError(LineError),
}

impl Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ParseError::*;

        match self {
            Empty => write!(f, "Tried to read from an empty file or string"),
            KnotError(err) => write!(f, "{}", err),
            LineError(err) => write!(f, "{}", err),
        }
    }
}

impl From<KnotError> for ParseError {
    fn from(err: KnotError) -> Self {
        ParseError::KnotError(err)
    }
}

impl From<LineError> for ParseError {
    fn from(err: LineError) -> Self {
        ParseError::LineError(err)
    }
}

#[derive(Debug)]
pub enum KnotError {
    /// Knot has no content.
    Empty,
    /// Could not parse a name for the knot. The offending string is encapsulated.
    InvalidName { line: String, kind: KnotNameError },
}

impl fmt::Display for KnotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use KnotError::Empty as EmptyKnot;
        use KnotError::*;
        use KnotNameError::Empty as EmptyKnotName;
        use KnotNameError::*;

        write!(f, "Could not parse a knot: ")?;

        match self {
            EmptyKnot => write!(f, "knot has no name"),
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
                }

                write!(f, " (line: {})", line)
            }
        }
    }
}

#[derive(Debug)]
pub enum KnotNameError {
    ContainsInvalidCharacter(char),
    ContainsWhitespace,
    Empty,
    NoNamePresent,
}

#[derive(Debug)]
pub enum LineError {
    /// Could not parse a condition.
    BadCondition {
        condition: String,
        full_line: String,
    },
    /// A line parsed as a choice has no set text to display as choice.
    NoDisplayText,
    /// A choice line contained both choice ('*') and sticky choice ('+') markers.
    MultipleChoiceType { line: String },
    /// Found unmatched brackets in a line.
    UnmatchedBrackets { line: String },
}

impl fmt::Display for LineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LineError::*;

        write!(f, "Invalid line: ")?;

        match self {
            BadCondition {
                condition,
                full_line,
            } => write!(
                f,
                "could not parse a condition from '{}' (full line: {})",
                condition, full_line,
            ),
            NoDisplayText => write!(
                f,
                "line has choice markers ({}, {}) but is empty",
                CHOICE_MARKER, STICKY_CHOICE_MARKER
            ),
            MultipleChoiceType { line } => write!(
                f,
                "line has multiple types of choice markers (line: {})",
                line
            ),
            UnmatchedBrackets { line } => write!(f, "line has unmatched brackets (line: {})", line),
        }
    }
}
