use crate::follow::{LineDataBuffer, Next};
use crate::line::*;

// #[cfg(test)]
use crate::error::ParseError;
// #[cfg(test)]
use std::str::FromStr;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

pub type ProcessError = String;

pub trait Process {
    fn process(&mut self, buffer: &mut String) -> Result<Next, ProcessError>;
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub struct FullLine {
    pub chunk: LineChunk,
    pub tags: Vec<String>,
    pub glue_begin: bool,
    pub glue_end: bool,
}

impl FullLine {
    pub fn process(&mut self, buffer: &mut LineDataBuffer) -> Result<Next, ProcessError> {
        let mut string = String::new();

        let result = self.chunk.process(&mut string);

        let mut full_line = parse_line(&string).unwrap();

        full_line.glue_begin = self.glue_begin;
        full_line.glue_end = self.glue_end;
        full_line.tags = self.tags.clone();

        buffer.push(full_line);

        result
    }

    pub fn text(&self) -> String {
        let mut buffer = String::new();

        for item in &self.chunk.items {
            match item {
                Content::PureText(string) => {
                    buffer.push_str(&string);
                }
                _ => (),
            }
        }

        buffer
    }

    pub fn from_chunk(chunk: LineChunk) -> Self {
        FullLine {
            chunk,
            tags: Vec::new(),
            glue_begin: false,
            glue_end: false,
        }
    }
}

pub struct FullLineBuilder {
    chunk: LineChunk,
    tags: Vec<String>,
    glue_begin: bool,
    glue_end: bool,
}

impl FullLineBuilder {
    pub fn from_chunk(chunk: LineChunk) -> Self {
        FullLineBuilder {
            chunk,
            tags: Vec::new(),
            glue_begin: false,
            glue_end: false,
        }
    }

    pub fn build(self) -> FullLine {
        FullLine {
            chunk: self.chunk,
            tags: self.tags,
            glue_begin: self.glue_begin,
            glue_end: self.glue_end,
        }
    }

    pub fn set_divert(&mut self, address: &str) {
        self.chunk.items.push(Content::Divert(address.to_string()));
    }

    pub fn set_glue_begin(&mut self, glue: bool) {
        self.glue_begin = glue;
    }

    pub fn set_glue_end(&mut self, glue: bool) {
        self.glue_end = glue;
    }

    pub fn set_tags(&mut self, tags: &[String]) {
        self.tags = tags.to_vec();
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub enum Content {
    Alternative(Alternative),
    Divert(String),
    Empty,
    PureText(String),
}

impl Process for Content {
    fn process(&mut self, buffer: &mut String) -> Result<Next, ProcessError> {
        match self {
            Content::Alternative(alternative) => alternative.process(buffer),
            Content::Divert(address) => Ok(Next::Divert(address.to_string())),
            Content::Empty => Ok(Next::Done),
            Content::PureText(string) => {
                buffer.push_str(string);
                Ok(Next::Done)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_line_processing_retains_glue() {
        let mut line = parse_line("A test string").unwrap();
        line.glue_begin = true;
        line.glue_end = true;

        let mut buffer = Vec::new();
        line.process(&mut buffer);

        let result = &buffer[0];
        assert!(result.glue_begin);
        assert!(result.glue_end);
    }

    #[test]
    fn full_line_processing_retains_tags() {
        let mut line = parse_line("A test string").unwrap();
        line.tags = vec!["tag 1".to_string(), "tag 2".to_string()];

        let mut buffer = Vec::new();
        line.process(&mut buffer);

        let result = &buffer[0];
        assert_eq!(result.tags, line.tags);
    }

    #[test]
    fn pure_text_line_processes_into_the_contained_string() {
        let mut buffer = String::new();

        Content::PureText("Hello, World!".to_string()).process(&mut buffer);

        assert_eq!(&buffer, "Hello, World!");
    }

    #[test]
    fn empty_content_does_not_process_into_anything() {
        let mut buffer = String::new();

        Content::Empty.process(&mut buffer);

        assert!(buffer.is_empty());
    }

    #[test]
    fn line_with_text_processes_into_that_text() {
        let content = "Text string.";

        let mut line = LineBuilder::new().with_text(content).unwrap().build();

        let mut buffer = String::new();

        line.process(&mut buffer).unwrap();

        assert_eq!(&buffer, content);
    }

    #[test]
    fn chunks_with_several_text_items_stitch_them_with_no_whitespace() {
        let mut line = LineBuilder::new()
            .with_text("Line 1")
            .unwrap()
            .with_text("Line 2")
            .unwrap()
            .build();

        let mut buffer = String::new();

        line.process(&mut buffer).unwrap();

        assert_eq!(&buffer, "Line 1Line 2");
    }

    #[test]
    fn lines_shortcut_if_proper_diverts_are_encountered() {
        let mut line = LineBuilder::new()
            .with_text("Line 1")
            .unwrap()
            .with_divert("divert")
            .with_text("Line 2")
            .unwrap()
            .build();

        let mut buffer = String::new();

        assert_eq!(
            line.process(&mut buffer).unwrap(),
            Next::Divert("divert".to_string())
        );

        assert_eq!(&buffer, "Line 1");
    }
}
