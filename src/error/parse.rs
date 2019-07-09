#[derive(Debug)]
/// Error from parsing text to construct a story.
pub enum ParseError {
    /// Attempted to construct a story from an empty file/string.
    Empty,
    /// Error from constructing a kot.
    KnotError(KnotError),
    /// Error from parsing a single line.
    LineError,
}

impl From<KnotError> for ParseError {
    fn from(err: KnotError) -> Self {
        ParseError::KnotError(err)
    }
}

#[derive(Debug)]
pub enum KnotError {
    /// Knot has no content.
    Empty,
    /// Could not parse a name for the knot. The offending string is encapsulated.
    NoName { string: String },
}
