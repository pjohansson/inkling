use crate::{
    follow::{EncounteredEvent, LineDataBuffer},
    line::{parse::parse_line, Content, InternalLine, LineChunk},
};

pub type ProcessError = String;

pub trait Process {
    fn process(&mut self, buffer: &mut String) -> Result<EncounteredEvent, ProcessError>;
}

impl InternalLine {
    pub fn process(
        &mut self,
        buffer: &mut LineDataBuffer,
    ) -> Result<EncounteredEvent, ProcessError> {
        let mut string = String::new();

        let result = self.chunk.process(&mut string);

        let mut full_line = parse_line(&string).unwrap();

        full_line.glue_begin = self.glue_begin;
        full_line.glue_end = self.glue_end;
        full_line.tags = self.tags.clone();

        buffer.push(full_line);

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
            Content::Divert(address) => Ok(EncounteredEvent::Divert(address.to_string())),
            Content::Empty => Ok(EncounteredEvent::Done),
            Content::Text(string) => {
                buffer.push_str(string);
                Ok(EncounteredEvent::Done)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::line::LineChunkBuilder;

    #[test]
    fn full_line_processing_retains_glue() {
        let mut line = parse_line("A test string").unwrap();
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
        let mut line = parse_line("A test string").unwrap();
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
    fn empty_content_does_not_process_into_anything() {
        let mut buffer = String::new();

        Content::Empty.process(&mut buffer).unwrap();

        assert!(buffer.is_empty());
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
            EncounteredEvent::Divert("divert".to_string())
        );

        assert_eq!(&buffer, "Line 1");
    }
}