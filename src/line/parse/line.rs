//! Parse `InternalLine` and `LineChunk` objects.

use crate::{
    consts::{DIVERT_MARKER, GLUE_MARKER, TAG_MARKER},
    line::{
        parse::{parse_alternative, split_line_at_separator, split_line_into_variants, LinePart},
        Content, InternalLine, InternalLineBuilder, LineChunk, LineChunkBuilder,
    },
};

#[derive(Clone, Debug)]
pub struct LineParsingError {
    pub line: String,
    pub kind: LineErrorKind,
}

impl LineParsingError {
    pub fn from_kind<T: Into<String>>(line: T, kind: LineErrorKind) -> Self {
        LineParsingError {
            line: line.into(),
            kind,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LineErrorKind {
    BlankChoice,
    EmptyDivert,
    ExpectedEndOfLine { tail: String },
    ExpectedLogic { line: String },
    ExpectedNumber { value: String },
    FoundTunnel,
    InvalidAddress { address: String },
    StickyAndNonSticky,
    UnmatchedBrackets,
    UnmatchedBraces,
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
    let mut builder = LineChunkBuilder::new();

    for part in split_line_into_variants(content)? {
        match part {
            LinePart::Text(part) => {
                add_text_items(part, &mut builder)?;
            }
            LinePart::Embraced(text) => {
                let alternative = parse_alternative(text)?;
                builder.add_item(Content::Alternative(alternative));
            }
        }
    }

    Ok(builder.build())
}

/// Parse and add text and divert items to a `LineChunkBuilder`.
fn add_text_items(content: &str, builder: &mut LineChunkBuilder) -> Result<(), LineParsingError> {
    let mut buffer = content.to_string();
    let divert = split_off_end_divert(&mut buffer)?;

    if buffer.trim().is_empty() {
        builder.add_item(Content::Empty);
    } else {
        builder.add_text(&buffer);
    }

    if let Some(address) = divert {
        builder.add_divert(&address);
    }

    Ok(())
}

/// Parse and remove glue markers from either side.
///
/// Enclosed whitespace within these markers is retained. Markers that are placed further
/// in are not (currently) removed.
fn parse_line_glue(line: &mut String, has_divert: bool) -> (bool, bool) {
    let glue_left = line.starts_with(GLUE_MARKER);
    let glue_right = line.ends_with(GLUE_MARKER);

    if glue_left {
        *line = line.trim_start_matches(GLUE_MARKER).to_string();
    }

    if glue_right {
        *line = line.trim_end_matches(GLUE_MARKER).to_string();
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

    let splits = split_line_at_separator(&line, DIVERT_MARKER)?;

    match splits.len() {
        0 | 1 => Ok(None),
        2 => {
            let head_length = splits.get(0).unwrap().len();

            let address = validate_divert_address(splits[1].trim(), backup_line)?;
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

/// Validate that a divert address can be parsed.
///
/// # Notes
/// *   Expectes the input line to be trimmed of whitespace from the edges.
fn validate_divert_address(line: &str, backup_line: String) -> Result<String, LineParsingError> {
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

    use crate::line::process::tests::get_processed_string;

    #[test]
    fn simple_text_string_parses_into_chunk_with_single_item() {
        let chunk = parse_chunk("Hello, World!").unwrap();

        assert_eq!(chunk.items.len(), 1);
        assert_eq!(chunk.items[0], Content::Text("Hello, World!".to_string()));
    }

    #[test]
    fn empty_string_parses_into_empty_object() {
        let chunk = parse_chunk("").unwrap();
        assert_eq!(chunk.items[0], Content::Empty);
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

        assert_eq!(&get_processed_string(&mut chunk), "One");
        assert_eq!(&get_processed_string(&mut chunk), "Two");
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
            &Content::Divert("world".to_string())
        );
    }

    #[test]
    fn string_in_chunk_with_divert_marker_inside_braces_and_at_end_is_valid() {
        let chunk =
            parse_chunk("Hello, {One|Two -> not_world|three -> not_world} -> world").unwrap();

        assert_eq!(
            chunk.items.last().unwrap(),
            &Content::Divert("world".to_string())
        );
    }

    #[test]
    fn string_with_divert_marker_adds_divert_item_at_end() {
        let chunk = parse_chunk("Hello -> world").unwrap();

        assert_eq!(chunk.items[1], Content::Divert("world".to_string()));
    }

    #[test]
    fn string_with_just_a_divert_gets_empty_object_and_then_divert() {
        let chunk = parse_chunk("-> hello_world").unwrap();

        assert_eq!(chunk.items.len(), 2);
        assert_eq!(chunk.items[0], Content::Empty);
        assert_eq!(chunk.items[1], Content::Divert("hello_world".to_string()));
    }

    #[test]
    fn divert_addresses_may_contain_dots() {
        let chunk = parse_chunk("-> hello.world").unwrap();
        assert_eq!(
            chunk.items.last().unwrap(),
            &Content::Divert("hello.world".to_string())
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
        assert_eq!(line.chunk.items[1], Content::Divert("world".to_string()));
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
}
