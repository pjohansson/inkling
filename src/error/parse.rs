use std::{error::Error, fmt};

use crate::consts::{CHOICE_MARKER, STICKY_CHOICE_MARKER};

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

#[derive(Debug)]
pub enum KnotError {
    /// Knot has no content.
    Empty,
    /// Could not parse a name for the knot. The offending string is encapsulated.
    InvalidName { line: String, kind: KnotNameError },
    /// Could not parse a line inside a not.
    LineError(LineParsingError),
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

impl Error for ParseError {}
impl Error for KnotError {}
impl Error for LineParsingError {}

impl_from_error![
    ParseError;
    [KnotError, KnotError],
    [LineError, LineParsingError]
];

impl_from_error![
    KnotError;
    [LineError, LineParsingError]
];

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
            LineError(err) => write!(f, "{}", err),
        }
    }
}

impl fmt::Display for LineParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LineErrorKind::*;

        match self.kind {
            BlankChoice => write!(
                f,
                "Found a choice with no selection text for the user to see, but with text \
                 that will be added to the buffer when selected. If this is a fallback choice \
                 the line content should be an empty divert, after which the content follows: \n\
                 '->'\n\
                 {{content}}\n\
                 "
            ),
            EmptyDivert => write!(f, "Encountered a divert statement with no address",),
            ExpectedEndOfLine { ref tail } => write!(
                f,
                "Expected no more content after a divert statement address but found '{}'",
                tail
            ),
            ExpectedLogic { ref line } => write!(
                f,
                "Could not parse a conditional logic statement '{}'",
                line
            ),
            ExpectedNumber { ref value } => write!(f, "Could not parse a number from '{}'", value),
            FoundTunnel => write!(
                f,
                "Found multiple divert markers in a line. In the `Ink` language this indicates \
                 a `tunnel` for the story to pass through, but these are not yet implemented \
                 in `inkling`."
            ),
            InvalidAddress { ref address } => write!(
                f,
                "Found an invalid address to knot, stitch or variable '{}': \
                 contains invalid characters",
                address
            ),
            StickyAndNonSticky => write!(
                f,
                "Encountered a line which has both non-sticky ('{}') and sticky ('{}') \
                 choice markers. This is not allowed.",
                CHOICE_MARKER, STICKY_CHOICE_MARKER
            ),
            UnmatchedBraces => write!(f, "Line has unmatched curly '{{}}' braces"),
            UnmatchedBrackets => write!(f, "Choice line has unmatched square '[]' brackets"),
        }?;

        write!(f, " (line: {}", &self.line)
    }
}

#[derive(Clone, Debug)]
pub enum KnotNameError {
    ContainsInvalidCharacter(char),
    ContainsWhitespace,
    Empty,
    NoNamePresent,
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
    UnmatchedBraces,
    UnmatchedBrackets,
}
