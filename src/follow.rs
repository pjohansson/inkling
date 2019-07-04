/// Trait to follow a story through lines, diverts and choices.
pub trait Follow {
    fn follow(&self, buffer: &mut String) -> Next;
}

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum Next {
    /// Move on with the story.
    Done,
    /// Divert to a new knot with the given name.
    Divert(String),
}
