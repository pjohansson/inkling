//! Parse all kinds of lines as marked up `ParsedLineKind` objects.

use crate::{
    consts::DIVERT_MARKER,
    line::{
        parse::{parse_choice, parse_gather, parse_internal_line},
        InternalChoice, InternalLine, LineParsingError,
    },
};

#[derive(Clone, Debug, PartialEq)]
/// Representation of a parsed line of content. 
/// 
/// To construct the nested tree structure of branching choices and gather points 
/// we need information about which level every choice and gather line is at. 
/// 
/// This structure marks the actual data of choices and gathers with their level. 
pub enum ParsedLineKind {
    Choice {
        /// Nested level of choice.
        level: u32,
        /// Parsed data of choice.
        choice_data: InternalChoice,
    },
    Gather {
        /// Nested level of gather.
        level: u32,
        /// Parsed line of gather point.
        line: InternalLine,
    },
    /// Regular line of content.
    Line(InternalLine),
}

#[cfg(test)]
impl ParsedLineKind {
    /// Construct a `ParsedLineKind::Choice` object with given level and choice data.
    pub fn choice(level: u32, choice_data: InternalChoice) -> Self {
        ParsedLineKind::Choice { level, choice_data }
    }

    /// Construct a `ParsedLineKind::Gather` object with given level and line.
    pub fn gather(level: u32, line: InternalLine) -> Self {
        ParsedLineKind::Gather { level, line }
    }

    /// Construct a `ParsedLineKind::Line` object with given line.
    pub fn line(line: InternalLine) -> Self {
        ParsedLineKind::Line(line)
    }
}

/// Parse a line into a `ParsedLineKind` object.
pub fn parse_line_kind(content: &str) -> Result<ParsedLineKind, LineParsingError> {
    if let Some(choice) = parse_choice(content)? {
        Ok(choice)
    } else if let Some(gather) = parse_gather(content)? {
        Ok(gather)
    } else {
        let line = parse_internal_line(content)?;

        Ok(ParsedLineKind::Line(line))
    }
}

/// Count leading markers and return the number and a string without them.
pub fn parse_markers_and_text(line: &str, marker: char) -> Option<(u32, &str)> {
    if line.trim_start().starts_with(marker) {
        let (markers, line_text) = split_markers_from_text(line, marker);
        let num = markers.matches(|c| c == marker).count() as u32;

        Some((num, line_text))
    } else {
        None
    }
}

/// Split leading markers from a string and return both parts.
fn split_markers_from_text(line: &str, marker: char) -> (&str, &str) {
    let split_at = line.find(|c: char| !(c == marker || c.is_whitespace()));

    match split_at {
        Some(i) => line.split_at(i),
        None => (line, ""),
    }
}

/// Split a string at the divert marker and return both parts.
pub fn split_at_divert_marker(content: &str) -> (&str, &str) {
    if let Some(i) = content.find(DIVERT_MARKER) {
        content.split_at(i)
    } else {
        (content, "")
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn simple_line_parses_to_line() {
        let line = parse_line_kind("Hello, World!").unwrap();
        let comparison = parse_internal_line("Hello, World!").unwrap();

        assert_eq!(line, ParsedLineKind::Line(comparison));
    }

    #[test]
    fn line_with_choice_markers_parses_to_choice() {
        let line = parse_line_kind("* Hello, World!").unwrap();

        match line {
            ParsedLineKind::Choice { .. } => (),
            other => panic!("expected `ParsedLineKind::Choice` but got {:?}", other),
        }
    }

    #[test]
    fn line_with_gather_markers_parses_to_gather() {
        let line = parse_line_kind("- Hello, World!").unwrap();

        match line {
            ParsedLineKind::Gather { .. } => (),
            other => panic!("expected `ParsedLineKind::Gather` but got {:?}", other),
        }
    }

    #[test]
    fn choices_are_parsed_before_gathers() {
        let line = parse_line_kind("* - Hello, World!").unwrap();

        match line {
            ParsedLineKind::Choice { .. } => (),
            other => panic!("expected `ParsedLineKind::Choice` but got {:?}", other),
        }
    }
}
