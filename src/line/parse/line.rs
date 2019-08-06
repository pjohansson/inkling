//! Parse `InternalLine` and `LineChunk` objects.

use crate::{
    consts::{DIVERT_MARKER, GLUE_MARKER, TAG_MARKER},
    error::{LineErrorKind, LineParsingError},
    knot::Address,
    line::{
        parse::{
            parse_alternative, parse_expression, parse_line_condition,
            utils::{split_line_at_separator_braces, split_line_into_groups_braces, LinePart},
        },
        Content, InternalLine, InternalLineBuilder, LineChunk,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
/// Kinds of variable expressions in an `Ink` line of text.
enum VariableText {
    /// Set of `Alternative` objects which will be selected from.
    Alternative,
    /// Content which will be display if (or if not) conditions are fulfilled.
    Conditional,
    /// An expression to evaluate and display the result of.
    Expression,
}

/// Parse an `InternalLine` from a string.
pub fn parse_internal_line(content: &str) -> Result<InternalLine, LineParsingError> {
    let mut buffer = content.to_string();

    let tags = parse_tags(&mut buffer);
    let divert = split_off_end_divert(&mut buffer)?;

    let (glue_begin, glue_end) = parse_line_glue(&mut buffer, divert.is_some());

    let chunk = parse_chunk(&buffer)?;

    let mut builder = InternalLineBuilder::from_chunk(chunk);

    if let Some(address) = divert {
        builder.set_divert(&address);
    }

    builder.set_glue_begin(glue_begin);
    builder.set_glue_end(glue_end);
    builder.set_tags(&tags);

    Ok(builder.build())
}

/// Parse a `LineChunk` object from a string.
pub fn parse_chunk(content: &str) -> Result<LineChunk, LineParsingError> {
    Ok(LineChunk {
        condition: None,
        items: parse_line_content(content)?,
        else_items: Vec::new(),
    })
}

fn parse_line_content(content: &str) -> Result<Vec<Content>, LineParsingError> {
    split_line_into_groups_braces(content)?
        .into_iter()
        .map(|group| match group {
            LinePart::Text(part) => get_text_items(part),
            LinePart::Embraced(text) => parse_embraced_line(text).map(|item| vec![item]),
        })
        .collect::<Result<Vec<Vec<_>>, _>>()
        .map(|items| items.into_iter().flatten().collect())
}

/// Parse and add text and divert items to a `LineChunkBuilder`.
fn get_text_items(content: &str) -> Result<Vec<Content>, LineParsingError> {
    let mut buffer = content.to_string();
    let mut items = Vec::new();

    let divert = split_off_end_divert(&mut buffer)?;

    if !buffer.trim().is_empty() {
        items.push(Content::Text(buffer));
    } else {
        items.push(Content::Empty);
    }

    if let Some(address) = divert {
        items.push(Content::Divert(Address::Raw(address)));
    }

    Ok(items)
}

fn parse_embraced_line(content: &str) -> Result<Content, LineParsingError> {
    match determine_kind(content)? {
        VariableText::Alternative => {
            let alternative = parse_alternative(content)?;
            Ok(Content::Alternative(alternative))
        }
        VariableText::Conditional => {
            let (condition, true_content, false_content) = parse_line_condition(content)?;

            let chunk = LineChunk {
                condition: Some(condition),
                items: parse_line_content(true_content)?,
                else_items: false_content
                    .map(|content| parse_line_content(content))
                    .transpose()?
                    .unwrap_or(Vec::new()),
            };

            Ok(Content::Nested(chunk))
        }
        VariableText::Expression => {
            let expression = parse_expression(content)
                .map_err(|kind| LineParsingError::from_kind(content, kind.into()))?;

            Ok(Content::Expression(expression))
        }
    }
}

/// Determine which kind of variable content is in an embraced string.
fn determine_kind(content: &str) -> Result<VariableText, LineParsingError> {
    if content.trim().is_empty() {
        Err(LineParsingError::from_kind(
            content,
            LineErrorKind::EmptyExpression,
        ))
    } else if split_line_at_separator_braces(content, ":", Some(1))?.len() > 1 {
        Ok(VariableText::Conditional)
    } else if split_line_at_separator_braces(content, "|", Some(1))?.len() > 1 {
        Ok(VariableText::Alternative)
    } else {
        Ok(VariableText::Expression)
    }
}

/// Parse and remove glue markers from either side.
///
/// Enclosed whitespace within these markers is retained. Markers that are placed further
/// in are not (currently) removed.
fn parse_line_glue(line: &mut String, has_divert: bool) -> (bool, bool) {
    let glue_left = line.trim_start().starts_with(GLUE_MARKER);
    let glue_right = line.trim_end().ends_with(GLUE_MARKER);

    if glue_left {
        *line = line
            .trim_start()
            .trim_start_matches(GLUE_MARKER)
            .to_string();
    }

    if glue_right {
        *line = line.trim_end().trim_end_matches(GLUE_MARKER).to_string();
    }

    (glue_left, glue_right || has_divert)
}

/// Split any found tags off the given line and return them separately.
fn parse_tags(line: &mut String) -> Vec<String> {
    match line.find(TAG_MARKER) {
        Some(i) => {
            let part = line.split_off(i);

            part.trim_matches(TAG_MARKER)
                .split(TAG_MARKER)
                .map(|tag| tag.trim().to_string())
                .collect::<Vec<_>>()
        }
        None => Vec::new(),
    }
}

/// Split diverts off the given line and return it separately if found.
fn split_off_end_divert(line: &mut String) -> Result<Option<String>, LineParsingError> {
    let backup_line = line.clone();

    let splits = split_line_at_separator_braces(&line, DIVERT_MARKER, None)?;

    match splits.len() {
        0 | 1 => Ok(None),
        2 => {
            let head_length = splits.get(0).unwrap().len();

            let address = validate_address(splits[1].trim(), backup_line)?;
            line.truncate(head_length);
            line.push(' ');

            Ok(Some(address))
        }
        _ => Err(LineParsingError::from_kind(
            line.clone(),
            LineErrorKind::FoundTunnel,
        )),
    }
}

/// Validate that an address for a divert or variable can be parsed.
///
/// # Notes
/// *   Expectes the input line to be trimmed of whitespace from the edges.
pub fn validate_address(line: &str, backup_line: String) -> Result<String, LineParsingError> {
    if line.contains(|c: char| c.is_whitespace()) {
        let tail = line
            .splitn(2, |c: char| c.is_whitespace())
            .skip(1)
            .next()
            .unwrap()
            .to_string();

        Err(LineParsingError::from_kind(
            backup_line,
            LineErrorKind::ExpectedEndOfLine { tail },
        ))
    } else if line.is_empty() {
        Err(LineParsingError {
            kind: LineErrorKind::EmptyDivert,
            line: backup_line,
        })
    } else if line.contains(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')) {
        Err(LineParsingError {
            kind: LineErrorKind::InvalidAddress {
                address: line.to_string(),
            },
            line: backup_line,
        })
    } else {
        Ok(line.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        knot::Address,
        line::{expression::Operand, Variable},
        process::line::tests::get_processed_chunk,
    };

    #[test]
    fn simple_text_string_parses_into_chunk_with_single_item() {
        let chunk = parse_chunk("Hello, World!").unwrap();

        assert_eq!(chunk.items.len(), 1);
        assert_eq!(chunk.items[0], Content::Text("Hello, World!".to_string()));
    }

    #[test]
    fn empty_string_parses_into_empty_chunk() {
        let chunk = parse_chunk("").unwrap();
        assert_eq!(chunk.items.len(), 0);
    }

    #[test]
    fn chunk_parsing_does_not_trim_whitespace() {
        let line = "    Hello, World!       ";
        let chunk = parse_chunk(line).unwrap();

        assert_eq!(chunk.items[0], Content::Text(line.to_string()));
    }

    #[test]
    fn braces_denote_alternative_sequences_in_chunks() {
        let mut chunk = parse_chunk("{One|Two}").unwrap();

        assert_eq!(chunk.items.len(), 1);

        match &chunk.items[0] {
            Content::Alternative(..) => (),
            other => panic!("expected `Content::Alternative` but got {:?}", other),
        }

        assert_eq!(&get_processed_chunk(&mut chunk), "One");
        assert_eq!(&get_processed_chunk(&mut chunk), "Two");
    }

    #[test]
    fn internal_line_with_divert_before_more_content_yields_error() {
        match parse_internal_line("Hello, -> world and {One|Two -> not_world}!") {
            Err(LineParsingError {
                kind: LineErrorKind::ExpectedEndOfLine { tail },
                ..
            }) => {
                assert_eq!(&tail, "and {One|Two -> not_world}!");
            }
            other => panic!(
                "expected `LineErrorKind::ExpectedEndOfLine` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn string_in_internal_line_with_divert_marker_inside_braces_and_at_end_is_valid() {
        let line = parse_internal_line("Hello, {One|Two -> not_world|three -> not_world} -> world")
            .unwrap();

        assert_eq!(
            line.chunk.items.last().unwrap(),
            &Content::Divert(Address::Raw("world".to_string()))
        );
    }

    #[test]
    fn string_in_chunk_with_divert_marker_inside_braces_and_at_end_is_valid() {
        let chunk =
            parse_chunk("Hello, {One|Two -> not_world|three -> not_world} -> world").unwrap();

        assert_eq!(
            chunk.items.last().unwrap(),
            &Content::Divert(Address::Raw("world".to_string()))
        );
    }

    #[test]
    fn string_with_divert_marker_adds_divert_item_at_end() {
        let chunk = parse_chunk("Hello -> world").unwrap();

        assert_eq!(
            chunk.items[1],
            Content::Divert(Address::Raw("world".to_string()))
        );
    }

    #[test]
    fn string_with_just_a_divert_gets_empty_object_and_then_divert() {
        let chunk = parse_chunk("-> hello_world").unwrap();

        assert_eq!(chunk.items.len(), 2);
        assert_eq!(chunk.items[0], Content::Empty);
        assert_eq!(
            chunk.items[1],
            Content::Divert(Address::Raw("hello_world".to_string()))
        );
    }

    #[test]
    fn divert_addresses_may_contain_dots() {
        let chunk = parse_chunk("-> hello.world").unwrap();
        assert_eq!(
            chunk.items.last().unwrap(),
            &Content::Divert(Address::Raw("hello.world".to_string()))
        );
    }

    #[test]
    fn divert_marker_adds_whitespace_to_the_left_of_it() {
        let chunk = parse_chunk("hello-> world").unwrap();
        assert_eq!(chunk.items[0], Content::Text("hello ".to_string()))
    }

    #[test]
    fn empty_divert_address_yields_error() {
        match parse_chunk("-> ") {
            Err(LineParsingError {
                kind: LineErrorKind::EmptyDivert,
                line,
            }) => {
                assert_eq!(line, "-> ");
            }
            other => panic!("expected `LineParsingError` but got {:?}", other),
        }
    }

    #[test]
    fn multiple_diverts_in_a_chunk_yields_error() {
        match parse_chunk("-> hello -> world") {
            Err(LineParsingError {
                kind: LineErrorKind::FoundTunnel,
                ..
            }) => (),
            other => panic!("expected `LineParsingError` but got {:?}", other),
        }
    }

    #[test]
    fn divert_address_must_be_valid() {
        match parse_chunk("-> hello$world") {
            Err(LineParsingError {
                kind: LineErrorKind::InvalidAddress { address },
                ..
            }) => assert_eq!(&address, "hello$world"),
            other => panic!("expected `LineParsingError` but got {:?}", other),
        }
    }

    #[test]
    fn divert_address_must_be_a_single_word() {
        match parse_chunk("-> hello world") {
            Err(LineParsingError {
                kind: LineErrorKind::ExpectedEndOfLine { tail },
                ..
            }) => assert_eq!(&tail, "world"),
            other => panic!("expected `LineParsingError` but got {:?}", other),
        }
    }

    #[test]
    fn glue_markers_add_glue_on_either_side_of_a_full_line() {
        let line = parse_internal_line("Hello, World!").unwrap();
        assert!(!line.glue_begin);
        assert!(!line.glue_end);

        let line = parse_internal_line("<> Hello, World!").unwrap();
        assert!(line.glue_begin);
        assert!(!line.glue_end);

        let line = parse_internal_line("Hello, World! <>").unwrap();
        assert!(!line.glue_begin);
        assert!(line.glue_end);

        let line = parse_internal_line("<> Hello, World! <>").unwrap();
        assert!(line.glue_begin);
        assert!(line.glue_end);
    }

    #[test]
    fn glue_markers_are_trimmed_from_line() {
        let line = parse_internal_line("<> Hello, World! <>").unwrap();
        assert_eq!(
            line.chunk.items[0],
            Content::Text(" Hello, World! ".to_string())
        );
    }

    #[test]
    fn diverts_are_parsed_if_there_is_glue() {
        let line = parse_internal_line("Hello <> -> world").unwrap();
        assert_eq!(
            line.chunk.items[1],
            Content::Divert(Address::Raw("world".to_string()))
        );
    }

    #[test]
    fn diverts_act_as_glue_for_full_line() {
        let line = parse_internal_line("Hello -> world").unwrap();
        assert!(line.glue_end);
    }

    #[test]
    fn tags_are_split_off_from_string_and_added_to_full_line_when_parsed() {
        let line = parse_internal_line("Hello, World! # tag one # tag two").unwrap();

        assert_eq!(line.tags.len(), 2);
        assert_eq!(&line.tags[0], "tag one");
        assert_eq!(&line.tags[1], "tag two");

        assert_eq!(line.chunk.items.len(), 1);
        assert_eq!(
            line.chunk.items[0],
            Content::Text("Hello, World! ".to_string())
        );
    }

    #[test]
    fn parse_embraced_line_as_alternative() {
        match parse_embraced_line("One | Two").unwrap() {
            Content::Alternative(..) => (),
            other => panic!("expected `Content::Alternative` but got {:?}", other),
        }
    }

    #[test]
    fn parse_embraced_line_as_new_conditional_chunk() {
        match parse_embraced_line("condition: One | Two").unwrap() {
            Content::Nested(chunk) => {
                let (condition, _, _) = parse_line_condition("condition: One | Two").unwrap();
                assert_eq!(chunk.condition.unwrap(), condition);
            }
            other => panic!("expected `Content::Nested` but got {:?}", other),
        }
    }

    #[test]
    fn parse_embraced_line_with_variable_parses_as_expression() {
        match parse_embraced_line("root").unwrap() {
            Content::Expression(expression) => {
                let address = Address::Raw("root".to_string());
                assert_eq!(
                    expression.head,
                    Operand::Variable(Variable::Address(address))
                );
            }
            other => panic!(
                "expected `Content::Nested(Variable::Address)` but got {:?}",
                other
            ),
        }

        match parse_embraced_line("root.stitch").unwrap() {
            Content::Expression(expression) => {
                let address = Address::Raw("root.stitch".to_string());
                assert_eq!(
                    expression.head,
                    Operand::Variable(Variable::Address(address))
                );
            }
            other => panic!(
                "expected `Content::Nested(Variable::Address)` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn parse_embraced_line_expression() {
        match parse_embraced_line("2 + 3").unwrap() {
            Content::Expression(expression) => {
                assert_eq!(expression.head, Operand::Variable(2.into()));
            }
            other => panic!("expected `Content::Expression` but got {:?}", other),
        }
    }

    #[test]
    fn address_in_embraced_variable_must_be_valid() {
        assert!(parse_embraced_line("root stitch").is_err());
        assert!(parse_embraced_line("root$stitch").is_err());
    }

    #[test]
    fn expression_with_colon_separator_is_condition() {
        assert_eq!(
            determine_kind("knot: item").unwrap(),
            VariableText::Conditional
        );
        assert_eq!(
            determine_kind("knot: item | item 2").unwrap(),
            VariableText::Conditional
        );
        assert_eq!(
            determine_kind("not knot : item").unwrap(),
            VariableText::Conditional
        );
        assert_eq!(
            determine_kind("not knot > 2 : item").unwrap(),
            VariableText::Conditional
        );
        assert_eq!(
            determine_kind("not knot > 2 : rest of line | another line").unwrap(),
            VariableText::Conditional
        );
    }

    #[test]
    fn expression_with_only_vertical_separators_is_alternative() {
        assert_eq!(
            determine_kind("one | two").unwrap(),
            VariableText::Alternative
        );
        assert_eq!(
            determine_kind("one|two").unwrap(),
            VariableText::Alternative
        );
    }

    #[test]
    fn expression_with_mathematical_operators_is_expression() {
        assert_eq!(determine_kind("+").unwrap(), VariableText::Expression);
        assert_eq!(determine_kind("-").unwrap(), VariableText::Expression);
        assert_eq!(determine_kind("*").unwrap(), VariableText::Expression);
        assert_eq!(determine_kind("/").unwrap(), VariableText::Expression);
        assert_eq!(determine_kind("%").unwrap(), VariableText::Expression);
    }

    #[test]
    fn empty_expression_will_yield_error() {
        assert!(determine_kind("").is_err());
    }
}
