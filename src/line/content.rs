use super::{
    condition::Condition,
    line::{LineData, LineKind, *},
    parse::*,
};

use crate::follow::{LineDataBuffer, Next};

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
                Content::Text(line) => {
                    buffer.push_str(&line.text);
                }
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
pub struct LineChunk {
    pub conditions: Vec<Condition>,
    pub items: Vec<Content>,
}

impl Process for LineChunk {
    fn process(&mut self, buffer: &mut String) -> Result<Next, ProcessError> {
        for item in self.items.iter_mut() {
            let result = item.process(buffer)?;

            if let Next::Divert(..) = result {
                return Ok(result);
            }
        }

        Ok(Next::Done)
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub enum Content {
    Alternative(Alternative),
    Divert(String),
    Empty,
    Text(LineData),
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
            Content::Text(line_data) => {
                buffer.push_str(&line_data.text);

                match &line_data.kind {
                    LineKind::Divert(address) => Ok(Next::Divert(address.to_string())),
                    LineKind::Regular => Ok(Next::Done),
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub struct Alternative {
    current_index: Option<usize>,
    kind: AlternativeKind,
    items: Vec<LineChunk>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
enum AlternativeKind {
    Cycle,
    OnceOnly,
    Sequence,
}

impl Process for Alternative {
    fn process(&mut self, buffer: &mut String) -> Result<Next, ProcessError> {
        let num_items = self.items.len();

        match self.kind {
            AlternativeKind::Cycle => {
                let index = self.current_index.get_or_insert(0);

                let item = self.items.get_mut(*index).ok_or(ProcessError::default())?;

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
                    None => Ok(Next::Done),
                }
            }
            AlternativeKind::Sequence => {
                let index = self.current_index.get_or_insert(0);

                let item = self.items.get_mut(*index).ok_or(ProcessError::default())?;

                if *index < num_items - 1 {
                    *index += 1;
                }

                item.process(buffer)
            }
        }
    }
}

pub struct LineBuilder {
    items: Vec<Content>,
}

impl LineBuilder {
    pub fn new() -> Self {
        LineBuilder { items: Vec::new() }
    }

    pub fn build(self) -> LineChunk {
        LineChunk {
            conditions: Vec::new(),
            items: self.items,
        }
    }

    pub fn add_pure_text(&mut self, text: &str) {
        self.add_item(Content::PureText(text.to_string()));
    }

    pub fn add_divert(&mut self, address: &str) {
        self.add_item(Content::Divert(address.to_string()));
    }

    pub fn add_item(&mut self, item: Content) {
        self.items.push(item);
    }

    pub fn with_divert(mut self, address: &str) -> Self {
        self.with_item(Content::Divert(address.to_string()))
    }

    pub fn with_item(mut self, item: Content) -> Self {
        self.items.push(item);
        self
    }

    pub fn with_line(mut self, line: LineData) -> Self {
        self.with_item(Content::Text(line))
    }

    pub fn with_pure_text(mut self, text: &str) -> Self {
        self.with_item(Content::PureText(text.to_string()))
    }

    // #[cfg(test)]
    pub fn with_text(mut self, text: &str) -> Result<Self, ParseError> {
        LineData::from_str(text)
            .map(|line_data| Content::Text(line_data))
            .map(|item| self.with_item(item))
    }
}

pub struct AlternativeBuilder {
    kind: AlternativeKind,
    items: Vec<LineChunk>,
}

impl AlternativeBuilder {
    fn from_kind(kind: AlternativeKind) -> Self {
        AlternativeBuilder {
            kind,
            items: Vec::new(),
        }
    }

    pub fn build(self) -> Alternative {
        Alternative {
            current_index: None,
            kind: self.kind,
            items: self.items,
        }
    }

    pub fn cycle() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::Cycle)
    }

    pub fn once_only() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::OnceOnly)
    }

    pub fn sequence() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::Sequence)
    }

    pub fn with_line(mut self, line: LineChunk) -> Self {
        self.items.push(line);
        self
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
    fn sequence_alternative_walks_through_content_when_processed_repeatably() {
        let mut sequence = AlternativeBuilder::sequence()
            .with_line(LineBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineBuilder::new().with_text("Line 2").unwrap().build())
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
            .with_line(LineBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineBuilder::new().with_text("Line 2").unwrap().build())
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
            .with_line(LineBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineBuilder::new().with_text("Line 2").unwrap().build())
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

    #[test]
    fn diverts_in_alternates_shortcut_when_finally_processed() {
        let mut alternative = AlternativeBuilder::sequence()
            .with_line(LineBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineBuilder::new().with_divert("divert").build())
            .with_line(LineBuilder::new().with_text("Line 2").unwrap().build())
            .build();

        let mut buffer = String::new();

        assert_eq!(alternative.process(&mut buffer).unwrap(), Next::Done);
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        assert_eq!(
            alternative.process(&mut buffer).unwrap(),
            Next::Divert("divert".to_string())
        );
        buffer.clear();

        assert_eq!(alternative.process(&mut buffer).unwrap(), Next::Done);
        assert_eq!(&buffer, "Line 2");
    }

    #[test]
    fn diverts_are_raised_through_the_nested_stack_when_encountered() {
        let alternative = AlternativeBuilder::sequence()
            .with_line(
                LineBuilder::new()
                    .with_text("Alternative line 1")
                    .unwrap()
                    .build(),
            )
            .with_line(
                LineBuilder::new()
                    .with_text("Divert")
                    .unwrap()
                    .with_divert("divert")
                    .build(),
            )
            .with_line(
                LineBuilder::new()
                    .with_text("Alternative line 2")
                    .unwrap()
                    .build(),
            )
            .build();

        let mut line = LineBuilder::new()
            .with_text("Line 1")
            .unwrap()
            .with_item(Content::Alternative(alternative))
            .with_text("Line 2")
            .unwrap()
            .build();

        let mut buffer = String::new();

        assert_eq!(line.process(&mut buffer).unwrap(), Next::Done);

        assert_eq!(&buffer, "Line 1Alternative line 1Line 2");
        buffer.clear();

        assert_eq!(
            line.process(&mut buffer).unwrap(),
            Next::Divert("divert".to_string())
        );

        assert_eq!(&buffer, "Line 1Divert");
        buffer.clear();

        assert_eq!(line.process(&mut buffer).unwrap(), Next::Done);

        assert_eq!(&buffer, "Line 1Alternative line 2Line 2");
    }
}
