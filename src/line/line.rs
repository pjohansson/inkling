//! Structures for representing a single, whole line of `Ink` content.

use crate::{
    error::{parse::validate::ValidationError, utils::MetaData},
    knot::Address,
    line::{Alternative, Condition, Expression},
    story::validate::{ValidateContent, ValidationData},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Representation of a single line of Ink content.
///
/// All of its raw data will be processed into a final form and presented to the user
/// as the story is followed.
pub struct InternalLine {
    /// Root chunk of line content, which may possibly be nested into even finer parts.
    pub chunk: LineChunk,
    /// Tags associated with the line. Will be given to the user along with the processed
    /// line content as the story is followed.
    pub tags: Vec<String>,
    /// Whether or not the line is glued to the previous line. Glue prohibits new lines
    /// to be added between lines, which is otherwise the default behavior when following
    /// the story.
    pub glue_begin: bool,
    /// Whether or not the line is glued to the next line.
    pub glue_end: bool,
    /// Information about the origin of this line in the story file or text.
    pub meta_data: MetaData,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Line content is nested into these smaller chunks.
///
/// When the chunk is processed it will, in order, process all child items. The simplest
/// example is a line of text with a divert. This can be represented as two items in this chunk.
/// When the chunk is processed the line content will be visited first, then
/// the divert will be encountered and returned through the call stack.
///
/// A more complicated example is a line which contains a set of variational content, which
/// in turn contain their own text, diverts and further nested variations. This necessitates
/// that the content in line is split into chunks like this.
///
/// Chunks possibly come with conditions for when the content will be visited
/// and displayed to the user.
pub struct LineChunk {
    /// ConditionKinds that must be fulfilled for the content to be processed.
    ///
    /// The conditions represent the entire chunk of items. If they are fulfilled, all items
    /// will be processed. If not, the chunk will be skipped during processing.
    pub condition: Option<Condition>,
    /// Set of line content which will be processed in order.
    pub items: Vec<Content>,
    /// Set of line content which will be processed in order if a condition is set and is `false`.
    pub else_items: Vec<Content>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Items in each chunk of line content comes in these forms.
pub enum Content {
    /// Content that alternates every time it is visited in the story.
    Alternative(Alternative),
    /// Divert to a new node in the story.
    Divert(Address),
    /// Null content.
    Empty,
    /// Expression to evaluate.
    Expression(Expression),
    /// Nested `LineChunk` to evaluate.
    Nested(LineChunk),
    /// String of regular text content in the line.
    Text(String),
}

impl InternalLine {
    /// Create the line from a finished chunk of line content.
    ///
    /// Will not set any tags or glue to the object.
    pub fn from_chunk(chunk: LineChunk) -> Self {
        InternalLine {
            chunk,
            tags: Vec::new(),
            glue_begin: false,
            glue_end: false,
            meta_data: MetaData { line_index: 0 },
        }
    }

    /// Get the text content from the lines direct children.
    ///
    /// TODO: Replace with a proper function once we finalize how `InternalLine` is processed.
    pub fn text(&self) -> String {
        let mut buffer = String::new();

        for item in &self.chunk.items {
            match item {
                Content::Text(string) => {
                    buffer.push_str(&string);
                }
                _ => (),
            }
        }

        buffer
    }

    #[cfg(test)]
    pub fn from_string(line: &str) -> Self {
        use builders::LineChunkBuilder;

        let chunk = LineChunkBuilder::from_string(line).build();
        Self::from_chunk(chunk)
    }
}

impl ValidateContent for InternalLine {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        current_location: &Address,
        _: &MetaData,
        data: &ValidationData,
    ) {
        self.chunk
            .validate(error, current_location, &self.meta_data, data);
    }
}

impl ValidateContent for LineChunk {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        current_location: &Address,
        meta_data: &MetaData,
        data: &ValidationData,
    ) {
        if let Some(condition) = self.condition.as_mut() {
            condition.validate(error, current_location, meta_data, data);
        }

        self.items
            .iter_mut()
            .for_each(|item| item.validate(error, current_location, meta_data, data));
    }
}

impl ValidateContent for Content {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        current_location: &Address,
        meta_data: &MetaData,
        data: &ValidationData,
    ) {
        match self {
            Content::Alternative(alternative) => {
                alternative.validate(error, current_location, meta_data, data)
            }
            Content::Divert(address) => address.validate(error, current_location, meta_data, data),
            Content::Empty | Content::Text(..) => (),
            Content::Expression(expression) => {
                expression.validate(error, current_location, meta_data, data)
            }
            Content::Nested(chunk) => chunk.validate(error, current_location, meta_data, data),
        }
    }
}

#[cfg(test)]
pub mod builders {
    //! Builders for line structures.

    use super::*;

    /// Builder for constructing an `InternalLine`.
    pub struct InternalLineBuilder {
        chunk: LineChunk,
        tags: Vec<String>,
        glue_begin: bool,
        glue_end: bool,
    }

    impl InternalLineBuilder {
        /// Construct the builder with an initial chunk of line content.
        pub fn from_chunk(chunk: LineChunk) -> Self {
            InternalLineBuilder {
                chunk,
                tags: Vec::new(),
                glue_begin: false,
                glue_end: false,
            }
        }

        /// Finalize the `InternalLine` object and return it.
        pub fn build(self) -> InternalLine {
            InternalLine {
                chunk: self.chunk,
                tags: self.tags,
                glue_begin: self.glue_begin,
                glue_end: self.glue_end,
                meta_data: MetaData { line_index: 0 },
            }
        }
    }

    /// Builder for constructing a `LineChunk`.
    ///
    /// # Notes
    /// *   If no items were added to the chunk, a `Content::Empty` item will be filled in.
    pub struct LineChunkBuilder {
        items: Vec<Content>,
    }

    impl LineChunkBuilder {
        /// Create an empty builder.
        pub fn new() -> Self {
            LineChunkBuilder { items: Vec::new() }
        }

        /// Finalize the `LineChunk` object and return it.
        pub fn build(mut self) -> LineChunk {
            if self.items.is_empty() {
                self.items.push(Content::Empty);
            }

            LineChunk {
                condition: None,
                items: self.items,
                else_items: Vec::new(),
            }
        }

        pub fn with_alternative(self, alternative: Alternative) -> Self {
            self.with_item(Content::Alternative(alternative))
        }

        pub fn with_divert(self, address: &str) -> Self {
            self.with_item(Content::Divert(Address::Raw(address.to_string())))
        }

        pub fn with_item(mut self, item: Content) -> Self {
            self.items.push(item);
            self
        }

        pub fn with_text(self, text: &str) -> Self {
            self.with_item(Content::Text(text.to_string()))
        }

        pub fn from_string(line: &str) -> Self {
            LineChunkBuilder::new().with_text(line)
        }
    }
}
