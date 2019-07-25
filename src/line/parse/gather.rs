use crate::{
    consts::GATHER_MARKER,
    line::{
        parse_line, parse_markers_and_text, split_at_divert_marker, LineParsingError,
        ParsedLineKind,
    },
};

pub fn parse_gather(content: &str) -> Result<Option<ParsedLineKind>, LineParsingError> {
    let (line_without_divert, line_from_divert) = split_at_divert_marker(content);

    parse_markers_and_text(line_without_divert, GATHER_MARKER)
        .map(|(level, remaining_text)| (level, format!("{}{}", remaining_text, line_from_divert)))
        .map(|(level, line)| parse_line(&line).map(|line| ParsedLineKind::Gather { level, line }))
        .transpose()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::line::{parse_line_kind, Content, FullLine};

    #[test]
    fn line_with_gather_markers_sets_line_text() {
        match parse_line_kind("- Hello, World!").unwrap() {
            ParsedLineKind::Gather { line, .. } => {
                assert_eq!(line, FullLine::from_string("Hello, World!"))
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line_kind("-- Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 2),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line_kind("------ Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 6),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn line_with_gather_markers_counts_them() {
        match parse_line_kind("- Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 1),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line_kind("-- Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 2),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line_kind("------ Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 6),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn line_with_gather_markers_ignores_whitespace() {
        match parse_line_kind("   - - -- Hello, World!").unwrap() {
            ParsedLineKind::Gather { level, .. } => assert_eq!(level, 4),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn gather_markers_do_not_require_text() {
        match parse_line_kind("-").unwrap() {
            ParsedLineKind::Gather { line, .. } => {
                assert_eq!(line.chunk.items.len(), 1);
                assert_eq!(line.chunk.items[0], Content::Empty);
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }

        match parse_line_kind(" - -  ").unwrap() {
            ParsedLineKind::Gather { line, .. } => {
                assert_eq!(line.chunk.items.len(), 1);
                assert_eq!(line.chunk.items[0], Content::Empty);
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn diverts_can_come_directly_after_gathers() {
        match parse_line_kind("- -> world").unwrap() {
            ParsedLineKind::Gather { line, .. } => {
                assert_eq!(line.chunk.items[0], Content::Empty);
                assert_eq!(line.chunk.items[1], Content::Divert("world".to_string()));
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn line_with_beginning_divert_parses_into_line_instead_of_gather() {
        match parse_line_kind("  -> world").unwrap() {
            ParsedLineKind::Line(line) => {
                assert_eq!(line.chunk.items[1], Content::Divert("world".to_string()));
            }
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }
}
