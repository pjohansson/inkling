use crate::follow::*;
use crate::line::*;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

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
    fn sequence_alternative_walks_through_content_when_processed_repeatably() {
        let mut sequence = AlternativeBuilder::sequence()
            .with_line(LineChunkBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineChunkBuilder::new().with_text("Line 2").unwrap().build())
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
            .with_line(LineChunkBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineChunkBuilder::new().with_text("Line 2").unwrap().build())
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
            .with_line(LineChunkBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineChunkBuilder::new().with_text("Line 2").unwrap().build())
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
            .with_line(LineChunkBuilder::new().with_text("Line 1").unwrap().build())
            .with_line(LineChunkBuilder::new().with_divert("divert").build())
            .with_line(LineChunkBuilder::new().with_text("Line 2").unwrap().build())
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
                LineChunkBuilder::new()
                    .with_text("Alternative line 1")
                    .unwrap()
                    .build(),
            )
            .with_line(
                LineChunkBuilder::new()
                    .with_text("Divert")
                    .unwrap()
                    .with_divert("divert")
                    .build(),
            )
            .with_line(
                LineChunkBuilder::new()
                    .with_text("Alternative line 2")
                    .unwrap()
                    .build(),
            )
            .build();

        let mut line = LineChunkBuilder::new()
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
