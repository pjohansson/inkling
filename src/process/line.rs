//! Processing of nested line chunks into text content.

use crate::{
    error::{ProcessError, ProcessErrorKind},
    follow::{EncounteredEvent, LineDataBuffer, LineText},
    line::{Alternative, AlternativeKind, Content, InternalLine, LineChunk},
};

pub trait Process {
    fn process(&mut self, buffer: &mut String) -> Result<EncounteredEvent, ProcessError>;
}

impl InternalLine {
    pub fn process(
        &mut self,
        buffer: &mut LineDataBuffer,
    ) -> Result<EncounteredEvent, ProcessError> {
        let mut text_buffer = String::new();

        let result = self.chunk.process(&mut text_buffer);

        let line_text = LineText {
            text: text_buffer,
            glue_begin: self.glue_begin,
            glue_end: self.glue_end,
            tags: self.tags.clone(),
        };

        buffer.push(line_text);

        result
    }
}

impl Process for LineChunk {
    fn process(&mut self, buffer: &mut String) -> Result<EncounteredEvent, ProcessError> {
        for item in self.items.iter_mut() {
            let result = item.process(buffer)?;

            if let EncounteredEvent::Divert(..) = result {
                return Ok(result);
            }
        }

        Ok(EncounteredEvent::Done)
    }
}

impl Process for Content {
    fn process(&mut self, buffer: &mut String) -> Result<EncounteredEvent, ProcessError> {
        match self {
            Content::Alternative(alternative) => alternative.process(buffer),
            Content::Divert(address) => Ok(EncounteredEvent::Divert(address.clone())),
            Content::Empty => {
                buffer.push(' ');
                Ok(EncounteredEvent::Done)
            }
            Content::Nested(chunk) => chunk.process(buffer),
            Content::Text(string) => {
                buffer.push_str(string);
                Ok(EncounteredEvent::Done)
            }
        }
    }
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

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::{
        knot::Address,
        line::{parse::parse_internal_line, AlternativeBuilder, LineChunkBuilder},
    };

    /// Process an item into a buffer an return it.
    pub fn get_processed_string<T: Process>(item: &mut T) -> String {
        let mut buffer = String::new();
        item.process(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn full_line_processing_retains_glue() {
        let mut line = parse_internal_line("A test string").unwrap();
        line.glue_begin = true;
        line.glue_end = true;

        let mut buffer = Vec::new();
        line.process(&mut buffer).unwrap();

        let result = &buffer[0];
        assert!(result.glue_begin);
        assert!(result.glue_end);
    }

    #[test]
    fn full_line_processing_retains_tags() {
        let mut line = parse_internal_line("A test string").unwrap();
        line.tags = vec!["tag 1".to_string(), "tag 2".to_string()];

        let mut buffer = Vec::new();
        line.process(&mut buffer).unwrap();

        let result = &buffer[0];
        assert_eq!(result.tags, line.tags);
    }

    #[test]
    fn pure_text_line_processes_into_the_contained_string() {
        let mut buffer = String::new();

        Content::Text("Hello, World!".to_string())
            .process(&mut buffer)
            .unwrap();

        assert_eq!(&buffer, "Hello, World!");
    }

    #[test]
    fn empty_content_processes_into_single_white_space() {
        let mut buffer = String::new();

        Content::Empty.process(&mut buffer).unwrap();

        assert_eq!(&buffer, " ");
    }

    #[test]
    fn line_with_text_processes_into_that_text() {
        let content = "Text string.";

        let mut line = LineChunkBuilder::from_string(content).build();

        let mut buffer = String::new();

        line.process(&mut buffer).unwrap();

        assert_eq!(&buffer, content);
    }

    #[test]
    fn chunks_with_several_text_items_stitch_them_with_no_whitespace() {
        let mut line = LineChunkBuilder::new()
            .with_text("Line 1")
            .with_text("Line 2")
            .build();

        let mut buffer = String::new();

        line.process(&mut buffer).unwrap();

        assert_eq!(&buffer, "Line 1Line 2");
    }

    #[test]
    fn lines_shortcut_if_proper_diverts_are_encountered() {
        let mut line = LineChunkBuilder::new()
            .with_text("Line 1")
            .with_divert("divert")
            .with_text("Line 2")
            .build();

        let mut buffer = String::new();

        assert_eq!(
            line.process(&mut buffer).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );

        assert_eq!(&buffer, "Line 1");
    }

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
