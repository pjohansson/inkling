//! Processing of nested line chunks into text content.

use crate::{
    error::ProcessError,
    follow::{EncounteredEvent, LineDataBuffer, LineText},
    line::{Content, InternalLine, LineChunk},
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
            Content::Text(string) => {
                buffer.push_str(string);
                Ok(EncounteredEvent::Done)
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::{
        line::{parse::parse_internal_line, LineChunkBuilder},
        story::Address,
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
}
