use super::{
    condition::Condition,
    line::{LineData, LineKind},
};

use crate::{
    error::ParseError,
    follow::{LineDataBuffer, Next},
};

use std::str::FromStr;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

pub type ProcessError = String;

pub trait Process {
    fn process(&mut self, buffer: &mut LineDataBuffer) -> Result<Next, ProcessError>;
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub struct Line {
    conditions: Vec<Condition>,
    items: Vec<Container>,
}

impl Process for Line {
    fn process(&mut self, buffer: &mut LineDataBuffer) -> Result<Next, ProcessError> {
        for item in self.items.iter_mut() {
            let result = item.process(buffer)?;

            if let Next::Divert(..) = result {
                return Ok(result);
            }
        }

        Ok(Next::Done)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub enum Container {
    Alternative(Alternative),
    Text(LineData),
}

impl Process for Container {
    fn process(&mut self, buffer: &mut LineDataBuffer) -> Result<Next, ProcessError> {
        match self {
            Container::Alternative(alternative) => alternative.process(buffer),
            Container::Text(line_data) => {
                buffer.push(line_data.clone());

                match &line_data.kind {
                    LineKind::Divert(address) => Ok(Next::Divert(address.to_string())),
                    LineKind::Regular => Ok(Next::Done),
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub struct Alternative {
    current_index: Option<usize>,
    kind: AlternativeKind,
    items: Vec<Line>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
enum AlternativeKind {
    Cycle,
    OnceOnly,
    Sequence,
}

impl Process for Alternative {
    fn process(&mut self, buffer: &mut LineDataBuffer) -> Result<Next, ProcessError> {
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
    items: Vec<Container>,
}

impl LineBuilder {
    pub fn new() -> Self {
        LineBuilder { items: Vec::new() }
    }

    pub fn build(self) -> Line {
        Line {
            conditions: Vec::new(),
            items: self.items,
        }
    }

    pub fn with_item(mut self, item: Container) -> Self {
        self.items.push(item);
        self
    }

    pub fn with_line(mut self, line: LineData) -> Self {
        self.items.push(Container::Text(line));
        self
    }

    pub fn with_text(mut self, text: &str) -> Result<Self, ParseError> {
        let item = Container::Text(LineData::from_str(text)?);
        Ok(self.with_item(item))
    }
}

pub struct AlternativeBuilder {
    kind: AlternativeKind,
    items: Vec<Line>,
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

    pub fn with_line(mut self, line: Line) -> Self {
        self.items.push(line);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_with_text_processes_into_that_text() {
        let content = "Text string.";

        let mut line = LineBuilder::new().with_text(content).unwrap().build();

        let mut buffer = Vec::new();

        line.process(&mut buffer).unwrap();

        assert_eq!(&buffer[0].text, content);
    }

    #[test]
    fn sequence_alternative_walks_through_content_when_processed_repeatably() {
        let mut sequence = AlternativeBuilder::sequence()
            .with_line(LineBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineBuilder::new().with_text("Line 2").unwrap().build())
            .build();

        let mut buffer = Vec::new();

        sequence.process(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 1);

        sequence.process(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 2);

        sequence.process(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 3);

        assert_eq!(&buffer[0].text, "Line 1");
        assert_eq!(&buffer[1].text, "Line 2");
        assert_eq!(&buffer[2].text, "Line 2");
    }

    #[test]
    fn once_only_alternative_walks_through_content_and_stops_after_final_item_when_processed() {
        let mut once_only = AlternativeBuilder::once_only()
            .with_line(LineBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineBuilder::new().with_text("Line 2").unwrap().build())
            .build();

        let mut buffer = Vec::new();

        once_only.process(&mut buffer).unwrap();
        once_only.process(&mut buffer).unwrap();
        once_only.process(&mut buffer).unwrap();

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, "Line 1");
        assert_eq!(&buffer[1].text, "Line 2");
    }

    #[test]
    fn cycle_alternative_repeats_from_first_index_after_reaching_end() {
        let mut cycle = AlternativeBuilder::cycle()
            .with_line(LineBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineBuilder::new().with_text("Line 2").unwrap().build())
            .build();

        let mut buffer = Vec::new();

        cycle.process(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 1);

        cycle.process(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 2);

        cycle.process(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 3);

        assert_eq!(&buffer[0].text, "Line 1");
        assert_eq!(&buffer[1].text, "Line 2");
        assert_eq!(&buffer[2].text, "Line 1");
    }

    #[test]
    fn lines_shortcut_if_a_divert_is_encountered() {
        let mut line = LineBuilder::new()
            .with_text("Line 1")
            .unwrap()
            .with_text("Divert -> divert")
            .unwrap()
            .with_text("Line 2")
            .unwrap()
            .build();

        let mut buffer = Vec::new();

        assert_eq!(
            line.process(&mut buffer).unwrap(),
            Next::Divert("divert".to_string())
        );

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer[0].text.trim(), "Line 1");
        assert_eq!(buffer[1].text.trim(), "Divert");
    }

    #[test]
    fn diverts_in_alternates_shortcut_when_finally_processed() {
        let mut alternative = AlternativeBuilder::sequence()
            .with_line(LineBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(
                LineBuilder::new()
                    .with_text("Divert -> divert")
                    .unwrap()
                    .build(),
            )
            .with_line(LineBuilder::new().with_text("Line 2").unwrap().build())
            .build();

        let mut buffer = Vec::new();

        assert_eq!(alternative.process(&mut buffer).unwrap(), Next::Done);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0].text.trim(), "Line 1");

        assert_eq!(
            alternative.process(&mut buffer).unwrap(),
            Next::Divert("divert".to_string())
        );
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer[1].text.trim(), "Divert");

        assert_eq!(alternative.process(&mut buffer).unwrap(), Next::Done);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[2].text.trim(), "Line 2");
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
                    .with_text("Divert -> divert")
                    .unwrap()
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
            .with_item(Container::Alternative(alternative))
            .with_text("Line 2")
            .unwrap()
            .build();

        let mut buffer = Vec::new();

        assert_eq!(line.process(&mut buffer).unwrap(), Next::Done);

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0].text.trim(), "Line 1");
        assert_eq!(buffer[1].text.trim(), "Alternative line 1");
        assert_eq!(buffer[2].text.trim(), "Line 2");

        buffer.clear();
        assert_eq!(
            line.process(&mut buffer).unwrap(),
            Next::Divert("divert".to_string())
        );

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer[0].text.trim(), "Line 1");
        assert_eq!(buffer[1].text.trim(), "Divert");

        buffer.clear();
        assert_eq!(line.process(&mut buffer).unwrap(), Next::Done);

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0].text.trim(), "Line 1");
        assert_eq!(buffer[1].text.trim(), "Alternative line 2");
        assert_eq!(buffer[2].text.trim(), "Line 2");
    }
}
