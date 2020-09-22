//! Processing of nested line chunks into text content.

use crate::{
    error::runtime::internal::{ProcessError, ProcessErrorKind},
    follow::{EncounteredEvent, FollowData, LineDataBuffer, LineText},
    line::{evaluate_expression, Alternative, Content, InternalLine, LineChunk},
    process::check_condition,
};

/// Process and add the content of an `InternalLine` to a buffer.
pub fn process_line(
    line: &mut InternalLine,
    buffer: &mut LineDataBuffer,
    data: &mut FollowData,
) -> Result<EncounteredEvent, ProcessError> {
    let mut text_buffer = String::new();

    let result = process_chunk(&mut line.chunk, &mut text_buffer, data);

    let line_text = LineText {
        text: text_buffer,
        glue_begin: line.glue_begin,
        glue_end: line.glue_end,
        tags: line.tags.clone(),
    };

    buffer.push(line_text);

    result
}

/// Process and add the content of a `LineChunk` to a string buffer.
///
/// If a condition is set to the chunk, it will be evaluated. If it evaluates to true,
/// the items in the `items` field will be processed. If not, the items in the `else_items`
/// field will be.
fn process_chunk(
    chunk: &mut LineChunk,
    buffer: &mut String,
    data: &mut FollowData,
) -> Result<EncounteredEvent, ProcessError> {
    let items = match &chunk.condition {
        Some(ref condition) => {
            if check_condition(condition, data)? {
                chunk.items.iter_mut()
            } else {
                chunk.else_items.iter_mut()
            }
        }
        None => chunk.items.iter_mut(),
    };

    for item in items {
        let result = process_content(item, buffer, data)?;

        if let EncounteredEvent::Divert(..) = result {
            return Ok(result);
        }
    }

    Ok(EncounteredEvent::Done)
}

/// Process and add the content of a `Content` item to a string buffer.
fn process_content(
    item: &mut Content,
    buffer: &mut String,
    data: &mut FollowData,
) -> Result<EncounteredEvent, ProcessError> {
    match item {
        Content::Alternative(alternative) => process_alternative(alternative, buffer, data),
        Content::Divert(address) => Ok(EncounteredEvent::Divert(address.clone())),
        Content::Empty => {
            buffer.push(' ');
            Ok(EncounteredEvent::Done)
        }
        Content::Expression(expression) => {
            let variable = evaluate_expression(&expression, data)?;
            buffer.push_str(&variable.to_string(data)?);
            Ok(EncounteredEvent::Done)
        }
        Content::Nested(chunk) => process_chunk(chunk, buffer, data),
        Content::Text(string) => {
            buffer.push_str(string);
            Ok(EncounteredEvent::Done)
        }
    }
}

/// Process and add the content of an `Alternative` to a string buffer.
fn process_alternative(
    alternative: &mut Alternative,
    buffer: &mut String,
    data: &mut FollowData,
) -> Result<EncounteredEvent, ProcessError> {
    match alternative.get_next_index(data) {
        Some(index) => {
            let item = alternative
                .items
                .get_mut(index)
                .ok_or_else(|| ProcessError {
                    kind: ProcessErrorKind::InvalidAlternativeIndex,
                })?;

            process_chunk(item, buffer, data)
        }
        None => Ok(EncounteredEvent::Done),
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::{
        follow::FollowDataBuilder,
        knot::Address,
        line::{
            expression::Operand, parse::parse_internal_line, AlternativeBuilder, ConditionBuilder,
            ConditionKind, Expression, LineChunkBuilder, Variable,
        },
    };

    use std::collections::HashMap;

    pub fn get_processed_alternative(alternative: &mut Alternative) -> String {
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        process_alternative(alternative, &mut buffer, &mut data).unwrap();

        buffer
    }

    pub fn get_processed_chunk(chunk: &mut LineChunk) -> String {
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        process_chunk(chunk, &mut buffer, &mut data).unwrap();

        buffer
    }

    pub fn mock_data_with_single_stitch(knot: &str, stitch: &str, num_visited: u32) -> FollowData {
        let mut stitch_count = HashMap::new();
        stitch_count.insert(stitch.to_string(), num_visited);

        let mut knot_visit_counts = HashMap::new();
        knot_visit_counts.insert(knot.to_string(), stitch_count);

        FollowDataBuilder::new()
            .with_knots(knot_visit_counts)
            .build()
    }

    #[test]
    fn full_line_processing_retains_glue() {
        let mut line = parse_internal_line("A test string", &().into()).unwrap();
        line.glue_begin = true;
        line.glue_end = true;

        let mut buffer = Vec::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        process_line(&mut line, &mut buffer, &mut data).unwrap();

        let result = &buffer[0];
        assert!(result.glue_begin);
        assert!(result.glue_end);
    }

    #[test]
    fn full_line_processing_retains_tags() {
        let mut line = parse_internal_line("A test string", &().into()).unwrap();
        line.tags = vec!["tag 1".to_string(), "tag 2".to_string()];

        let mut buffer = Vec::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        process_line(&mut line, &mut buffer, &mut data).unwrap();

        let result = &buffer[0];
        assert_eq!(result.tags, line.tags);
    }

    #[test]
    fn pure_text_line_processes_into_the_contained_string() {
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        let mut item = Content::Text("Hello, World!".to_string());
        process_content(&mut item, &mut buffer, &mut data).unwrap();

        assert_eq!(&buffer, "Hello, World!");
    }

    #[test]
    fn expression_evaluates_into_variable_and_prints_it() {
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        let expression = Expression {
            head: Operand::Variable(5.into()),
            tail: Vec::new(),
        };

        let mut item = Content::Expression(expression);

        process_content(&mut item, &mut buffer, &mut data).unwrap();

        assert_eq!(&buffer, "5");
    }

    #[test]
    fn divert_variable_yields_error() {
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        let variable = Variable::Divert(Address::End);

        let expression = Expression {
            head: Operand::Variable(variable),
            tail: Vec::new(),
        };

        let mut item = Content::Expression(expression);

        assert!(process_content(&mut item, &mut buffer, &mut data).is_err());
    }

    #[test]
    fn empty_content_processes_into_single_white_space() {
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        let mut item = Content::Empty;
        process_content(&mut item, &mut buffer, &mut data).unwrap();

        assert_eq!(&buffer, " ");
    }

    #[test]
    fn line_with_text_processes_into_that_text() {
        let content = "Text string.";
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        let mut line = LineChunkBuilder::from_string(content).build();
        process_chunk(&mut line, &mut buffer, &mut data).unwrap();

        assert_eq!(&buffer, content);
    }

    #[test]
    fn chunks_with_several_text_items_stitch_them_with_no_whitespace() {
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        let mut chunk = LineChunkBuilder::new()
            .with_text("Line 1")
            .with_text("Line 2")
            .build();

        process_chunk(&mut chunk, &mut buffer, &mut data).unwrap();

        assert_eq!(&buffer, "Line 1Line 2");
    }

    #[test]
    fn chunk_with_condition_processes_its_content_if_it_is_fulfilled() {
        let true_condition = ConditionBuilder::from_kind(&ConditionKind::True, false).build();
        let false_condition = ConditionBuilder::from_kind(&ConditionKind::False, false).build();

        let mut chunk = LineChunk {
            condition: Some(true_condition),
            items: vec![Content::Text("Displayed if true.".to_string())],
            else_items: Vec::new(),
        };

        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);
        process_chunk(&mut chunk, &mut buffer, &mut data).unwrap();

        assert_eq!(&buffer, "Displayed if true.");

        chunk.condition.replace(false_condition);

        buffer.clear();
        process_chunk(&mut chunk, &mut buffer, &mut data).unwrap();
        assert_eq!(&buffer, "");
    }

    #[test]
    fn if_a_condition_is_false_content_in_the_else_items_list_is_processed() {
        let true_condition = ConditionBuilder::from_kind(&ConditionKind::True, false).build();
        let false_condition = ConditionBuilder::from_kind(&ConditionKind::False, false).build();

        let mut chunk = LineChunk {
            condition: Some(true_condition),
            items: vec![Content::Text("Displayed if true.".to_string())],
            else_items: vec![Content::Text("Displayed if false.".to_string())],
        };

        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);
        process_chunk(&mut chunk, &mut buffer, &mut data).unwrap();

        assert_eq!(&buffer, "Displayed if true.");

        chunk.condition.replace(false_condition);

        buffer.clear();
        process_chunk(&mut chunk, &mut buffer, &mut data).unwrap();
        assert_eq!(&buffer, "Displayed if false.");
    }

    #[test]
    fn chunks_without_condition_always_processes_the_true_content() {
        let mut chunk = LineChunk {
            condition: None,
            items: vec![Content::Text("Displayed if true.".to_string())],
            else_items: vec![Content::Text("Displayed if false.".to_string())],
        };

        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);
        process_chunk(&mut chunk, &mut buffer, &mut data).unwrap();

        assert_eq!(&buffer, "Displayed if true.");
    }

    #[test]
    fn lines_shortcut_if_proper_diverts_are_encountered() {
        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        let mut chunk = LineChunkBuilder::new()
            .with_text("Line 1")
            .with_divert("divert")
            .with_text("Line 2")
            .build();

        assert_eq!(
            process_chunk(&mut chunk, &mut buffer, &mut data).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );

        assert_eq!(&buffer, "Line 1");
    }

    #[test]
    fn diverts_in_alternates_shortcut_when_finally_processed() {
        let mut alternative = AlternativeBuilder::sequence()
            .with_line(LineChunkBuilder::from_string("Line 1").build())
            .with_line(LineChunkBuilder::new().with_divert("divert").build())
            .with_line(LineChunkBuilder::from_string("Line 2").build())
            .build();

        let mut buffer = String::new();
        let mut data = mock_data_with_single_stitch("", "", 0);

        assert_eq!(
            process_alternative(&mut alternative, &mut buffer, &mut data).unwrap(),
            EncounteredEvent::Done
        );
        assert_eq!(&buffer, "Line 1");
        buffer.clear();

        assert_eq!(
            process_alternative(&mut alternative, &mut buffer, &mut data).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );
        buffer.clear();

        assert_eq!(
            process_alternative(&mut alternative, &mut buffer, &mut data).unwrap(),
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
        let mut data = mock_data_with_single_stitch("", "", 0);

        assert_eq!(
            process_chunk(&mut line, &mut buffer, &mut data).unwrap(),
            EncounteredEvent::Done
        );

        assert_eq!(&buffer, "Line 1Alternative line 1Line 2");
        buffer.clear();

        assert_eq!(
            process_chunk(&mut line, &mut buffer, &mut data).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );

        assert_eq!(&buffer, "Line 1Divert");
        buffer.clear();

        assert_eq!(
            process_chunk(&mut line, &mut buffer, &mut data).unwrap(),
            EncounteredEvent::Done
        );

        assert_eq!(&buffer, "Line 1Alternative line 2Line 2");
    }
}
