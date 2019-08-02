//! Content that alternates from a fixed set when processed.

use crate::{
    error::{InvalidAddressError, ProcessError, ProcessErrorKind},
    follow::EncounteredEvent,
    knot::KnotSet,
    line::{LineChunk, Process},
    story::{Address, ValidateAddresses},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Set of line content which can vary when it is processed.
///
/// The variational content comes from a fixed set of chunks. When the `Alternative`
/// is processed it will pick one item from this set and process it. Which item is
/// selected depends on which kind of alternative it is.
///
/// Any selected `LineChunk`s can of course contain nested alternatives, and so on.
pub struct Alternative {
    /// Current index in the set of content.
    current_index: Option<usize>,
    /// Which kind of alternative this represents.
    pub kind: AlternativeKind,
    /// Set of content which the object will select and process from.
    pub items: Vec<LineChunk>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Variants of alternating content.
pub enum AlternativeKind {
    /// Cycles through the set, starting from the beginning after reaching the end.
    ///
    /// # Example
    /// A set of the week days `[Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday]`
    /// will in turn print every day, then start over again from Monday after Sunday has been
    /// visited.
    Cycle,
    /// Goes through the set of content once, then produces nothing.
    ///
    /// # Example
    /// A countdown from `[Three, Two, One]` will print the numbers, then nothing after
    /// the last item has been shown.
    OnceOnly,
    /// Goes through the set of content once, then repeats the final item.
    ///
    /// # Example
    /// A train traveling to its destination `[Frankfurt, Mannheim, Heidelberg]` will print
    /// each destination, then `Heidelberg` forever after reaching the city.
    Sequence,
}

impl Process for Alternative {
    fn process(&mut self, buffer: &mut String) -> Result<EncounteredEvent, ProcessError> {
        let num_items = self.items.len();

        match self.kind {
            AlternativeKind::Cycle => {
                let index = self.current_index.get_or_insert(0);

                let item = self.items.get_mut(*index).ok_or_else(|| ProcessError {
                    kind: ProcessErrorKind::InvalidAlternativeIndex,
                })?;

                if *index < num_items - 1 {
                    *index += 1;
                } else {
                    *index = 0;
                }

                item.process(buffer)
            }
            AlternativeKind::OnceOnly => {
                let index = self.current_index.get_or_insert(0);

                match self.items.get_mut(*index) {
                    Some(item) => {
                        *index += 1;
                        item.process(buffer)
                    }
                    None => Ok(EncounteredEvent::Done),
                }
            }
            AlternativeKind::Sequence => {
                let index = self.current_index.get_or_insert(0);

                let item = self.items.get_mut(*index).ok_or_else(|| ProcessError {
                    kind: ProcessErrorKind::InvalidAlternativeIndex,
                })?;

                if *index < num_items - 1 {
                    *index += 1;
                }

                item.process(buffer)
            }
        }
    }
}

impl ValidateAddresses for Alternative {
    fn validate(
        &mut self,
        current_address: &Address,
        knots: &KnotSet,
    ) -> Result<(), InvalidAddressError> {
        self.items
            .iter_mut()
            .map(|item| item.validate(current_address, knots))
            .collect()
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        self.items.iter().all(|item| item.all_addresses_are_valid())
    }
}

/// Builder struct for `Alternative`.
pub struct AlternativeBuilder {
    kind: AlternativeKind,
    items: Vec<LineChunk>,
}

impl AlternativeBuilder {
    /// Construct the builder with the given `AlternativeKind`.
    pub fn from_kind(kind: AlternativeKind) -> Self {
        AlternativeBuilder {
            kind,
            items: Vec::new(),
        }
    }

    /// Finalize the `Alternative` and return it.
    pub fn build(self) -> Alternative {
        Alternative {
            current_index: None,
            kind: self.kind,
            items: self.items,
        }
    }

    /// Set the alternative `LineChunk`s to the builder.
    ///
    /// # Notes
    /// *   Replaces the current set of items.
    pub fn with_items(mut self, items: Vec<LineChunk>) -> Self {
        self.items = items;
        self
    }

    #[cfg(test)]
    /// Construct a builder with `AlternativeKind::Cycle`.
    pub fn cycle() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::Cycle)
    }

    #[cfg(test)]
    /// Construct a builder with `AlternativeKind::OnceOnly`.
    pub fn once_only() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::OnceOnly)
    }

    #[cfg(test)]
    /// Construct a builder with `AlternativeKind::Sequence`.
    pub fn sequence() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::Sequence)
    }

    #[cfg(test)]
    /// Add a chunk of line content to the set of alternatives.
    pub fn add_line(&mut self, line: LineChunk) {
        self.items.push(line);
    }

    #[cfg(test)]
    /// Add a chunk of line content to the set of alternatives.
    pub fn with_line(mut self, line: LineChunk) -> Self {
        self.add_line(line);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        line::{Content, LineChunkBuilder},
        story::Address,
    };

    #[test]
    fn sequence_alternative_walks_through_content_when_processed_repeatably() {
        let mut sequence = AlternativeBuilder::sequence()
            .with_line(LineChunkBuilder::from_string("Line 1").build())
            .with_line(LineChunkBuilder::from_string("Line 2").build())
            .build();

        let mut buffer = String::new();

        sequence.process(&mut buffer).unwrap();
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        sequence.process(&mut buffer).unwrap();
        assert_eq!(&buffer, "Line 2");
        buffer.clear();

        sequence.process(&mut buffer).unwrap();
        assert_eq!(&buffer, "Line 2");
        buffer.clear();
    }

    #[test]
    fn once_only_alternative_walks_through_content_and_stops_after_final_item_when_processed() {
        let mut once_only = AlternativeBuilder::once_only()
            .with_line(LineChunkBuilder::from_string("Line 1").build())
            .with_line(LineChunkBuilder::from_string("Line 2").build())
            .build();

        let mut buffer = String::new();

        once_only.process(&mut buffer).unwrap();
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        once_only.process(&mut buffer).unwrap();
        assert_eq!(&buffer, "Line 2");
        buffer.clear();

        once_only.process(&mut buffer).unwrap();
        assert!(buffer.is_empty());
    }

    #[test]
    fn cycle_alternative_repeats_from_first_index_after_reaching_end() {
        let mut cycle = AlternativeBuilder::cycle()
            .with_line(LineChunkBuilder::from_string("Line 1").build())
            .with_line(LineChunkBuilder::from_string("Line 2").build())
            .build();

        let mut buffer = String::new();

        cycle.process(&mut buffer).unwrap();
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        cycle.process(&mut buffer).unwrap();
        assert_eq!(&buffer, "Line 2");
        buffer.clear();

        cycle.process(&mut buffer).unwrap();
        assert_eq!(&buffer, "Line 1");
        buffer.clear();
    }

    #[test]
    fn diverts_in_alternates_shortcut_when_finally_processed() {
        let mut alternative = AlternativeBuilder::sequence()
            .with_line(LineChunkBuilder::from_string("Line 1").build())
            .with_line(LineChunkBuilder::new().with_divert("divert").build())
            .with_line(LineChunkBuilder::from_string("Line 2").build())
            .build();

        let mut buffer = String::new();

        assert_eq!(
            alternative.process(&mut buffer).unwrap(),
            EncounteredEvent::Done
        );
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        assert_eq!(
            alternative.process(&mut buffer).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );
        buffer.clear();

        assert_eq!(
            alternative.process(&mut buffer).unwrap(),
            EncounteredEvent::Done
        );
        assert_eq!(&buffer, "Line 2");
    }

    #[test]
    fn diverts_are_raised_through_the_nested_stack_when_encountered() {
        let alternative = AlternativeBuilder::sequence()
            .with_line(LineChunkBuilder::from_string("Alternative line 1").build())
            .with_line(
                LineChunkBuilder::from_string("Divert")
                    .with_divert("divert")
                    .build(),
            )
            .with_line(LineChunkBuilder::from_string("Alternative line 2").build())
            .build();

        let mut line = LineChunkBuilder::new()
            .with_text("Line 1")
            .with_item(Content::Alternative(alternative))
            .with_text("Line 2")
            .build();

        let mut buffer = String::new();

        assert_eq!(line.process(&mut buffer).unwrap(), EncounteredEvent::Done);

        assert_eq!(&buffer, "Line 1Alternative line 1Line 2");
        buffer.clear();

        assert_eq!(
            line.process(&mut buffer).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );

        assert_eq!(&buffer, "Line 1Divert");
        buffer.clear();

        assert_eq!(line.process(&mut buffer).unwrap(), EncounteredEvent::Done);

        assert_eq!(&buffer, "Line 1Alternative line 2Line 2");
    }
}
