//! Parse `InternalLine` and `LineChunk` objects.

use crate::{
    consts::{DIVERT_MARKER, GLUE_MARKER, TAG_MARKER},
    line::{Content, InternalLine, InternalLineBuilder, LineChunk, LineChunkBuilder},
};

#[derive(Clone, Debug)]
pub struct LineParsingError {
    pub line: String,
    pub kind: LineErrorKind,
}

#[derive(Clone, Debug)]
pub enum LineErrorKind {
    BlankChoice,
    EmptyDivert,
    FoundTunnel,
    InvalidAddress { address: String },
    StickyAndNonSticky,
    UnmatchedBrackets,
}

/// Parse an `InternalLine` from a string.
pub fn parse_internal_line(content: &str) -> Result<InternalLine, LineParsingError> {
    let mut buffer = content.to_string();

    let tags = parse_tags(&mut buffer);
    let divert = parse_divert(&mut buffer)?;
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

    let mut buffer = content.to_string();
    let divert = parse_divert(&mut buffer)?;

    if buffer.trim().is_empty() {
        builder.add_item(Content::Empty);
    } else {
        builder.add_text(&buffer);
    }

    if let Some(address) = divert {
        builder.add_divert(&address);
    }

    Ok(builder.build())
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
fn parse_divert(line: &mut String) -> Result<Option<String>, LineParsingError> {
    let backup_line = line.clone();

    line.find(DIVERT_MARKER)
        .map(|i| {
            let divert_line = line.split_off(i);
            line.push(' ');

            let (_, tail) = divert_line.split_at(DIVERT_MARKER.len());

            verify_divert_address(tail.trim(), backup_line)
        })
        .transpose()
}

/// Validate that a divert address can be parsed.
fn verify_divert_address(line: &str, backup_line: String) -> Result<String, LineParsingError> {
    if line.contains(DIVERT_MARKER) {
        Err(LineParsingError {
            kind: LineErrorKind::FoundTunnel,
            line: backup_line,
        })
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
    fn divert_address_must_be_a_single_word() {
        match parse_chunk("-> hello world") {
            Err(LineParsingError {
                kind: LineErrorKind::InvalidAddress { address },
                ..
            }) => assert_eq!(&address, "hello world"),
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
