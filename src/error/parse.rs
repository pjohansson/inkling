use std::{error::Error, fmt};

use crate::{
    consts::{CHOICE_MARKER, STICKY_CHOICE_MARKER},
};

#[derive(Debug)]
/// Error from parsing text to construct a story.
pub enum ParseError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// Could not construct a `Knot` or `Stitch` as the content was read.
    KnotError(KnotError),
    /// Could not parse a individual line outside of knots.
    LineError(LineParsingError),
}

impl Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ParseError::*;

        match self {
            Empty => write!(f, "Tried to read from an empty file or string"),
            KnotError(err) => write!(f, "{}", err),
            LineError(err) => write!(f, "{:?}", err),
        }
    }
}

impl_from_error![
    ParseError;
    [KnotError, KnotError],
    [LineError, LineParsingError]
];

impl_from_error![
    KnotError;
    [LineError, LineParsingError]
];

#[derive(Debug)]
pub enum KnotError {
    /// Knot has no content.
    Empty,
    /// Could not parse a name for the knot. The offending string is encapsulated.
    InvalidName { line: String, kind: KnotNameError },
    /// Could not parse a line inside a not.
    LineError(LineParsingError),
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
            },
            LineError(err) => unimplemented!(),
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

#[derive(Clone, Debug)]
pub struct LineParsingError {
    pub line: String,
    pub kind: LineErrorKind,
}

impl LineParsingError {
    pub fn from_kind<T: Into<String>>(line: T, kind: LineErrorKind) -> Self {
        LineParsingError {
            line: line.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LineErrorKind {
    BlankChoice,
    EmptyDivert,
    ExpectedEndOfLine { tail: String },
    ExpectedLogic { line: String },
    ExpectedNumber { value: String },
    FoundTunnel,
    InvalidAddress { address: String },
    StickyAndNonSticky,
    UnmatchedBrackets,
    UnmatchedBraces,
}
