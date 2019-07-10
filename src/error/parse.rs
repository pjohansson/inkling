#[derive(Debug)]
/// Error from parsing text to construct a story.
pub enum ParseError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// Error from constructing a kot.
    KnotError(KnotError),
    /// Error from parsing a single line.
    LineError(LineError),
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
    NoName { string: String },
}

#[derive(Debug)]
pub enum LineError {
    /// A line parsed as a choice has no set text to display as choice.
    NoDisplayText,
    /// A choice line contained both choice ('*') and sticky choice ('+') markers.
    MultipleChoiceType { line: String },
}
