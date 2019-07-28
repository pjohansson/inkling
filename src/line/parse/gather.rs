//! Parse gathers as marked up `ParsedLineKind::Gather` objects.

use crate::{
    consts::GATHER_MARKER,
    error::LineParsingError,
    line::{
        parse::{parse_internal_line, parse_markers_and_text, split_at_divert_marker},
        ParsedLineKind,
    },
};

/// Parse a `ParsedLineKind::Gather` from a line if the line represents a gather point.
pub fn parse_gather(content: &str) -> Result<Option<ParsedLineKind>, LineParsingError> {
    let (line_without_divert, line_from_divert) = split_at_divert_marker(content);

    parse_markers_and_text(line_without_divert, GATHER_MARKER)
        .map(|(level, remaining_text)| (level, format!("{}{}", remaining_text, line_from_divert)))
        .map(|(level, line)| {
            parse_internal_line(&line).map(|line| ParsedLineKind::Gather { level, line })
        })
        .transpose()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::line::{parse_line, Content, InternalLine};

    #[test]
    fn line_with_gather_markers_sets_line_text() {
        match parse_line("- Hello, World!").unwrap() {
            ParsedLineKind::Gather { line, .. } => {
                assert_eq!(line, InternalLine::from_string("Hello, World!"))
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line("-- Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 2),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line("------ Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 6),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn line_with_gather_markers_counts_them() {
        match parse_line("- Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 1),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line("-- Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 2),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line("------ Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 6),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn line_with_gather_markers_ignores_whitespace() {
        match parse_line("   - - -- Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 4),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn gather_markers_do_not_require_text() {
        match parse_line("-").unwrap() {
            ParsedLineKind::Gather { line, .. } => {
                assert_eq!(line.chunk.items.len(), 1);
                assert_eq!(line.chunk.items[0], Content::Empty);
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line(" - -  ").unwrap() {
            ParsedLineKind::Gather { line, .. } => {
                assert_eq!(line.chunk.items.len(), 1);
                assert_eq!(line.chunk.items[0], Content::Empty);
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn diverts_can_come_directly_after_gathers() {
        match parse_line("- -> world").unwrap() {
            ParsedLineKind::Gather { line, .. } => {
                assert_eq!(line.chunk.items[0], Content::Empty);
                assert_eq!(line.chunk.items[1], Content::Divert("world".to_string()));
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn line_with_beginning_divert_parses_into_line_instead_of_gather() {
        match parse_line("  -> world").unwrap() {
            ParsedLineKind::Line(line) => {
                assert_eq!(line.chunk.items[1], Content::Divert("world".to_string()));
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }
}
