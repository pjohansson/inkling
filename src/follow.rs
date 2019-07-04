use crate::line::Line;

/// Trait to follow a story through lines, diverts and choices.
pub trait Follow {
    /// Follow a story while reading every line into a buffer.
    fn follow(&self, buffer: &mut LineBuffer) -> Next;

    /// Follow a story while reading every line into a pure text buffer,
    /// discarding other data.
    fn follow_into_string(&self, buffer: &mut String) -> Next {
        let mut line_buffer = LineBuffer::new();

        let result = self.follow(&mut line_buffer);

        for line in line_buffer {
            buffer.push_str(&line.text);
        }

        result
    }
}

pub type LineBuffer = Vec<LineContent>;

pub struct LineContent {
    pub text: String,
    pub tags: Vec<String>,
}

impl From<&Line> for LineContent {
    fn from(line: &Line) -> Self {
        LineContent {
            text: line.text.clone(),
            tags: line.tags.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum Next {
    /// Move on with the story.
    Line,
    /// Divert to a new knot with the given name.
    Divert(String),
    /// Choice for the user.
    Choice,
}
