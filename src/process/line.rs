//! Processing of nested line chunks into text content.

use crate::{
    error::{ProcessError, ProcessErrorKind},
    follow::{EncounteredEvent, LineDataBuffer, LineText},
    line::{Alternative, AlternativeKind, Content, InternalLine, LineChunk},
};

pub fn process_line(
    line: &mut InternalLine,
    buffer: &mut LineDataBuffer,
) -> Result<EncounteredEvent, ProcessError> {
    let mut text_buffer = String::new();

    let result = process_chunk(&mut line.chunk, &mut text_buffer);

    let line_text = LineText {
        text: text_buffer,
        glue_begin: line.glue_begin,
        glue_end: line.glue_end,
        tags: line.tags.clone(),
    };

    buffer.push(line_text);

    result
}

fn process_chunk(
    chunk: &mut LineChunk,
    buffer: &mut String,
) -> Result<EncounteredEvent, ProcessError> {
    for item in chunk.items.iter_mut() {
        let result = process_content(item, buffer)?;

        if let EncounteredEvent::Divert(..) = result {
            return Ok(result);
        }
    }

    Ok(EncounteredEvent::Done)
}

fn process_content(
    item: &mut Content,
    buffer: &mut String,
) -> Result<EncounteredEvent, ProcessError> {
    match item {
        Content::Alternative(alternative) => process_alternative(alternative, buffer),
        Content::Divert(address) => Ok(EncounteredEvent::Divert(address.clone())),
        Content::Empty => {
            buffer.push(' ');
            Ok(EncounteredEvent::Done)
        }
        Content::Nested(chunk) => process_chunk(chunk, buffer),
        Content::Text(string) => {
            buffer.push_str(string);
            Ok(EncounteredEvent::Done)
        }
    }
}

fn process_alternative(
    alternative: &mut Alternative,
    buffer: &mut String,
) -> Result<EncounteredEvent, ProcessError> {
    let num_items = alternative.items.len();

    match alternative.kind {
        AlternativeKind::Cycle => {
            let index = alternative.current_index.get_or_insert(0);

            let item = alternative
                .items
                .get_mut(*index)
                .ok_or_else(|| ProcessError {
                    kind: ProcessErrorKind::InvalidAlternativeIndex,
                })?;

            if *index < num_items - 1 {
                *index += 1;
            } else {
                *index = 0;
            }

            process_chunk(item, buffer)
        }
        AlternativeKind::OnceOnly => {
            let index = alternative.current_index.get_or_insert(0);

            match alternative.items.get_mut(*index) {
                Some(item) => {
                    *index += 1;
                    process_chunk(item, buffer)
                }
                None => Ok(EncounteredEvent::Done),
            }
        }
        AlternativeKind::Sequence => {
            let index = alternative.current_index.get_or_insert(0);

            let item = alternative
                .items
                .get_mut(*index)
                .ok_or_else(|| ProcessError {
                    kind: ProcessErrorKind::InvalidAlternativeIndex,
                })?;

            if *index < num_items - 1 {
                *index += 1;
            }

            process_chunk(item, buffer)
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

    pub fn get_processed_alternative(alternative: &mut Alternative) -> String {
        let mut buffer = String::new();
        process_alternative(alternative, &mut buffer).unwrap();
        buffer
    }

    pub fn get_processed_chunk(chunk: &mut LineChunk) -> String {
        let mut buffer = String::new();
        process_chunk(chunk, &mut buffer).unwrap();
        buffer
    }

    #[test]
    fn full_line_processing_retains_glue() {
        let mut line = parse_internal_line("A test string").unwrap();
        line.glue_begin = true;
        line.glue_end = true;

        let mut buffer = Vec::new();
        process_line(&mut line, &mut buffer).unwrap();

        let result = &buffer[0];
        assert!(result.glue_begin);
        assert!(result.glue_end);
    }

    #[test]
    fn full_line_processing_retains_tags() {
        let mut line = parse_internal_line("A test string").unwrap();
        line.tags = vec!["tag 1".to_string(), "tag 2".to_string()];

        let mut buffer = Vec::new();
        process_line(&mut line, &mut buffer).unwrap();

        let result = &buffer[0];
        assert_eq!(result.tags, line.tags);
    }

    #[test]
    fn pure_text_line_processes_into_the_contained_string() {
        let mut buffer = String::new();

        let mut item = Content::Text("Hello, World!".to_string());
        process_content(&mut item, &mut buffer).unwrap();

        assert_eq!(&buffer, "Hello, World!");
    }

    #[test]
    fn empty_content_processes_into_single_white_space() {
        let mut buffer = String::new();

        let mut item = Content::Empty;
        process_content(&mut item, &mut buffer).unwrap();

        assert_eq!(&buffer, " ");
    }

    #[test]
    fn line_with_text_processes_into_that_text() {
        let content = "Text string.";
        let mut buffer = String::new();

        let mut line = LineChunkBuilder::from_string(content).build();
        process_chunk(&mut line, &mut buffer).unwrap();

        assert_eq!(&buffer, content);
    }

    #[test]
    fn chunks_with_several_text_items_stitch_them_with_no_whitespace() {
        let mut buffer = String::new();

        let mut line = LineChunkBuilder::new()
            .with_text("Line 1")
            .with_text("Line 2")
            .build();

        process_chunk(&mut line, &mut buffer).unwrap();

        assert_eq!(&buffer, "Line 1Line 2");
    }

    #[test]
    fn lines_shortcut_if_proper_diverts_are_encountered() {
        let mut buffer = String::new();

        let mut line = LineChunkBuilder::new()
            .with_text("Line 1")
            .with_divert("divert")
            .with_text("Line 2")
            .build();

        assert_eq!(
            process_chunk(&mut line, &mut buffer).unwrap(),
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

        process_alternative(&mut sequence, &mut buffer).unwrap();
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        process_alternative(&mut sequence, &mut buffer).unwrap();
        assert_eq!(&buffer, "Line 2");
        buffer.clear();

        process_alternative(&mut sequence, &mut buffer).unwrap();
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

        process_alternative(&mut once_only, &mut buffer).unwrap();
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        process_alternative(&mut once_only, &mut buffer).unwrap();
        assert_eq!(&buffer, "Line 2");
        buffer.clear();

        process_alternative(&mut once_only, &mut buffer).unwrap();
        assert!(buffer.is_empty());
    }

    #[test]
    fn cycle_alternative_repeats_from_first_index_after_reaching_end() {
        let mut cycle = AlternativeBuilder::cycle()
            .with_line(LineChunkBuilder::from_string("Line 1").build())
            .with_line(LineChunkBuilder::from_string("Line 2").build())
            .build();

        let mut buffer = String::new();

        process_alternative(&mut cycle, &mut buffer).unwrap();
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        process_alternative(&mut cycle, &mut buffer).unwrap();
        assert_eq!(&buffer, "Line 2");
        buffer.clear();

        process_alternative(&mut cycle, &mut buffer).unwrap();
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
            process_alternative(&mut alternative, &mut buffer).unwrap(),
            EncounteredEvent::Done
        );
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        assert_eq!(
            process_alternative(&mut alternative, &mut buffer).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );
        buffer.clear();

        assert_eq!(
            process_alternative(&mut alternative, &mut buffer).unwrap(),
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

        assert_eq!(
            process_chunk(&mut line, &mut buffer).unwrap(),
            EncounteredEvent::Done
        );

        assert_eq!(&buffer, "Line 1Alternative line 1Line 2");
        buffer.clear();

        assert_eq!(
            process_chunk(&mut line, &mut buffer).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );

        assert_eq!(&buffer, "Line 1Divert");
        buffer.clear();

        assert_eq!(
            process_chunk(&mut line, &mut buffer).unwrap(),
            EncounteredEvent::Done
        );

        assert_eq!(&buffer, "Line 1Alternative line 2Line 2");
    }
}
