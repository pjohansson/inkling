use crate::{
    consts::DIVERT_MARKER,
    line::{
        parse::{parse_choice, parse_gather, parse_line},
        FullChoice, FullLine, LineParsingError,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum ParsedLineKind {
    Choice { level: u32, choice_data: FullChoice },
    Gather { level: u32, line: FullLine },
    Line(FullLine),
}

#[cfg(test)]
impl ParsedLineKind {
    pub fn choice(level: u32, choice_data: FullChoice) -> Self {
        ParsedLineKind::Choice { level, choice_data }
    }

    pub fn gather(level: u32, line: FullLine) -> Self {
        ParsedLineKind::Gather { level, line }
    }

    pub fn line(line: FullLine) -> Self {
        ParsedLineKind::Line(line)
    }
}

pub fn parse_line_kind(content: &str) -> Result<ParsedLineKind, LineParsingError> {
    if let Some(choice) = parse_choice(content)? {
        Ok(choice)
    } else if let Some(gather) = parse_gather(content)? {
        Ok(gather)
    } else {
        let line = parse_line(content)?;

        Ok(ParsedLineKind::Line(line))
    }
}

pub fn parse_markers_and_text(line: &str, marker: char) -> Option<(u32, &str)> {
    if line.trim_start().starts_with(marker) {
        let (markers, line_text) = split_markers_from_text(line, marker);
        let num = markers.matches(|c| c == marker).count() as u32;

        Some((num, line_text))
    } else {
        None
    }
}

fn split_markers_from_text(line: &str, marker: char) -> (&str, &str) {
    let split_at = line.find(|c: char| !(c == marker || c.is_whitespace()));

    match split_at {
        Some(i) => line.split_at(i),
        None => (line, ""),
    }
}

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
        let comparison = parse_line("Hello, World!").unwrap();

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
