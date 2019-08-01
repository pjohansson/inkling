//! Errors from parsing stories, knots, stitches and lines.

use std::{error::Error, fmt};

use crate::{
    consts::{CHOICE_MARKER, STICKY_CHOICE_MARKER},
    story::Address,
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
    /// An invalid address was encountered when parsing the story.
    InvalidAddress(InvalidAddressError),
}

#[derive(Clone, Debug)]
/// A divert (or other address) in the story is invalid.
pub enum InvalidAddressError {
    /// The address is not formatted correctly.
    BadFormat { line: String },
    /// Tried to validate an address but the given current knot did not exist in the system.
    UnknownCurrentAddress { address: Address },
    /// The address references a `Knot` that is not in the story.
    UnknownKnot { knot_name: String },
    /// The address references a `Stitch` that is not present in the current `Knot`.
    UnknownStitch {
        knot_name: String,
        stitch_name: String,
    },
    /// Tried to validate an address using an unvalidated current address.
    ValidatedWithUnvalidatedAddress {
        needle: String,
        current_address: Address,
    },
}

#[derive(Debug)]
/// Error from parsing a `Knot` or `Stitch` in a story.
pub enum KnotError {
    /// Knot has no content.
    Empty,
    /// Could not parse a name for the knot. The offending string is encapsulated.
    InvalidName { line: String, kind: KnotNameError },
    /// Could not parse a line inside a not.
    LineError(LineParsingError),
}

#[derive(Clone, Debug)]
/// Error from parsing individual lines in a story.
pub struct LineParsingError {
    /// Line that caused the error.
    pub line: String,
    /// Kind of error.
    pub kind: LineErrorKind,
}

impl LineParsingError {
    /// Constructor of error from some string and kind.
    pub fn from_kind<T: Into<String>>(line: T, kind: LineErrorKind) -> Self {
        LineParsingError {
            line: line.into(),
            kind,
        }
    }
}

impl Error for ParseError {}
impl Error for InvalidAddressError {}
impl Error for KnotError {}
impl Error for LineParsingError {}

impl_from_error![
    ParseError;
    [InvalidAddress, InvalidAddressError],
    [KnotError, KnotError],
    [LineError, LineParsingError]
];

impl_from_error![
    KnotError;
    [LineError, LineParsingError]
];

impl_from_error![
    LineErrorKind;
    [BadCondition, BadCondition]
];

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ParseError::*;

        match self {
            Empty => write!(f, "Tried to read from an empty file or string"),
            InvalidAddress(err) => write!(f, "{}", err),
            KnotError(err) => write!(f, "{}", err),
            LineError(err) => write!(f, "{}", err),
        }
    }
}

impl fmt::Display for InvalidAddressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use InvalidAddressError::*;

        write!(f, "Encountered an invalid address: ")?;

        match self {
            BadFormat { line } => write!(f, "address was incorrectly formatted ('{}')", line),
            UnknownCurrentAddress { address } => write!(
                f,
                "during validation an address '{:?}' that is not in the system was used as
                 a current address",
                address
            ),
            UnknownKnot { knot_name } => {
                write!(f, "no knot with name '{}' in the story", knot_name)
            }
            UnknownStitch {
                knot_name,
                stitch_name,
            } => write!(
                f,
                "no stitch with name '{}' in knot '{}'",
                stitch_name, knot_name
            ),
            ValidatedWithUnvalidatedAddress {
                needle,
                current_address,
            } => write!(
                f,
                "during validating the raw address '{}' an unvalidated address '{:?}' was used",
                needle, current_address
            ),
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

impl fmt::Display for LineParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LineErrorKind::*;

        match self.kind {
            BadCondition(..) => unimplemented!(),
            EmptyDivert => write!(f, "Encountered a divert statement with no address",),
            EmptyExpression => write!(f, "Found an empty embraced expression ({{}})"),
            ExpectedEndOfLine { ref tail } => write!(
                f,
                "Expected no more content after a divert statement address but found '{}'",
                tail
            ),
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

#[derive(Clone, Debug)]
/// Variants of line errors.
pub enum LineErrorKind {
    /// Condition was invalid.
    BadCondition(BadCondition),
    /// Found a divert marker but no address.
    EmptyDivert,
    /// Found an empty expression (embraced part of line)
    EmptyExpression,
    /// Line did not end after a divert statement.
    ExpectedEndOfLine { tail: String },
    /// Found several divert markers which indicates unimplemented tunnels.
    FoundTunnel,
    /// Found an address with invalid characters.
    InvalidAddress { address: String },
    /// A choice has both non-sticky and sticky markers.
    StickyAndNonSticky,
    /// Found unmatched curly braces.
    UnmatchedBraces,
    /// Found unmatched square brackets.
    UnmatchedBrackets,
}

#[derive(Clone, Debug)]
pub struct BadCondition {
    content: String,
    kind: BadConditionKind,
}

impl BadCondition {
    pub fn from_kind<T: Into<String>>(content: T, kind: BadConditionKind) -> Self {
        BadCondition {
            content: content.into(),
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BadConditionKind {
    /// The first item in a condition was not `Blank` or any other item was not `And` or `Or`.
    ///
    /// This is an internal consistency check from parsing a condition. Every subsequent
    /// condition to the first should be preceeded by an `and` or `or` marker, while the
    /// first condition should not be. After parsing the condition we assert that this is true.
    /// If not, some internal shenanigans are going on, but this should be unreachable.
    BadLink,
    /// Could not parse a number from the condition.
    BadValue,
    /// Generic error.
    CouldNotParse,
    /// The line had multiple else statements.
    MultipleElseStatements,
    /// There was no condition in the line.
    NoCondition,
    /// Found unmatched parenthesis.
    UnmatchedParenthesis,
}
