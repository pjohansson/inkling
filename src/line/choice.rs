use crate::{
    consts::{
        CHOICE_MARKER, STICKY_CHOICE_MARKER,
    },
    error::{LineError, ParseError},
};

use std::str::FromStr;

use super::{
    condition::{parse_choice_conditions, Condition},
    line::{parse_markers_and_text, LineData, ParsedLine},
};

#[derive(Clone, Debug, PartialEq)]
/// A single choice in a (usually) set of choices presented to the user.
pub struct ChoiceData {
    /// Text presented to the user to represent the choice.
    pub displayed: LineData,
    /// Text that the choice produces when selected, replacing the `displayed` line.
    /// Can be empty, in which case the presented text is removed before the story flow
    /// continues to the next line.
    pub line: LineData,
    /// Number of times the choice has been selected so far in the story.
    pub num_visited: u32,
    /// By default a choice will be filtered after being visited once. If it is marked
    /// as sticky it will stick around.
    pub is_sticky: bool,
    /// Conditions that must be fulfilled for the choice to be displayed.
    pub conditions: Vec<Condition>,
}

pub fn parse_choice(line: &str) -> Option<Result<ParsedLine, ParseError>> {
    parse_choice_markers_and_text(line).map(|result| {
        result.and_then(|(level, is_sticky, line_text)| {
            prepare_parsed_choice_from_line(level, is_sticky, line_text)
        })
    })
}

fn prepare_parsed_choice_from_line(
    level: u8,
    is_sticky: bool,
    line: &str,
) -> Result<ParsedLine, ParseError> {
    let mut remaining_line = line.to_string();
    let conditions = parse_choice_conditions(&mut remaining_line)?;

    if remaining_line.is_empty() {
        return Err(LineError::NoDisplayText.into());
    }

    let (displayed_text, line_text) = parse_choice_line_variants(&remaining_line)?;

    let displayed = LineData::from_str(&displayed_text)?;
    let line = LineData::from_str(&line_text)?;

    let choice = ChoiceData {
        displayed,
        line,
        num_visited: 0,
        is_sticky,
        conditions,
    };

    Ok(ParsedLine::Choice { level, choice })
}

/// Split choice markers (sticky or non-sticky) from a line. If they are present, ensure
/// that the line does not have both sticky and non-sticky markers. Return the number
/// of markers along with whether the choice was sticky and the remaining line.
fn parse_choice_markers_and_text(line: &str) -> Option<Result<(u8, bool, &str), ParseError>> {
    let choice_parse = parse_markers_and_text(line, CHOICE_MARKER);
    let is_sticky = choice_parse.is_none();

    let (num_markers, remaining_line) =
        choice_parse.or_else(|| parse_markers_and_text(line, STICKY_CHOICE_MARKER))?;

    if remaining_line.starts_with(|c| c == CHOICE_MARKER || c == STICKY_CHOICE_MARKER) {
        return Some(Err(LineError::MultipleChoiceType {
            line: line.to_string(),
        }
        .into()));
    }

    Some(Ok((num_markers, is_sticky, remaining_line)))
}

fn parse_choice_line_variants(line: &str) -> Result<(String, String), ParseError> {
    match (line.find('['), line.find(']')) {
        (Some(i), Some(j)) if i < j => {
            // Ensure that we don't have more brackets
            if line.rfind('[').unwrap() != i || line.rfind(']').unwrap() != j {
                return Err(LineError::UnmatchedBrackets {
                    line: line.to_string(),
                }
                .into());
            }

            let head = line.get(..i).unwrap();
            let inside = line.get(i + 1..j).unwrap();
            let tail = line.get(j + 1..).unwrap();

            let displayed = format!("{}{}", head, inside);
            let line = format!("{}{}", head, tail);

            Ok((displayed, line))
        }
        (None, None) => Ok((line.to_string(), line.to_string())),
        _ => Err(LineError::UnmatchedBrackets {
            line: line.to_string(),
        }
        .into()),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    impl ChoiceData {
        pub fn empty() -> Self {
            ChoiceBuilder::empty().build()
        }
    }

    pub struct ChoiceBuilder {
        line: LineData,
        displayed: LineData,
        num_visited: u32,
        is_sticky: bool,
        conditions: Vec<Condition>,
    }

    impl ChoiceBuilder {
        pub fn empty() -> Self {
            let line = LineData::empty();

            ChoiceBuilder {
                displayed: line.clone(),
                line,
                num_visited: 0,
                is_sticky: false,
                conditions: Vec::new(),
            }
        }

        pub fn build(self) -> ChoiceData {
            ChoiceData {
                displayed: self.displayed,
                line: self.line,
                num_visited: self.num_visited,
                is_sticky: self.is_sticky,
                conditions: self.conditions,
            }
        }

        pub fn is_sticky(mut self) -> Self {
            self.is_sticky = true;
            self
        }

        pub fn with_conditions(mut self, conditions: &[Condition]) -> Self {
            self.conditions.extend_from_slice(conditions);
            self
        }

        pub fn with_displayed(mut self, line: LineData) -> Self {
            self.displayed = line;
            self
        }

        pub fn with_line(mut self, line: LineData) -> Self {
            self.line = line;
            self
        }

        pub fn with_num_visited(mut self, num_visited: u32) -> Self {
            self.num_visited = num_visited;
            self
        }
    }

    #[test]
    fn parsing_choice_line_variants_return_same_line_if_no_brackets_are_present() {
        let (displayed, line) = parse_choice_line_variants("Hello, World!").unwrap();
        assert_eq!(displayed, line);
    }

    #[test]
    fn parsing_choice_line_variants_break_the_displayed_line_when_encountering_square_brackets() {
        let (displayed, line) = parse_choice_line_variants("Hello[], World!").unwrap();
        assert_eq!(&displayed, "Hello");
        assert_eq!(&line, "Hello, World!");
    }

    #[test]
    fn parsing_choice_line_variants_include_content_inside_square_brackets_in_displayed() {
        let (displayed, line) = parse_choice_line_variants("Hello[!], World!").unwrap();
        assert_eq!(&displayed, "Hello!");
        assert_eq!(&line, "Hello, World!");
    }

    #[test]
    fn parsing_choice_line_variants_return_error_if_brackets_are_unmatched() {
        assert!(parse_choice_line_variants("Hello[!, World!").is_err());
        assert!(parse_choice_line_variants("Hello]!, World!").is_err());
    }

    #[test]
    fn parsing_choice_line_variants_return_error_more_brackets_are_found() {
        assert!(parse_choice_line_variants("Hello[!], [Worl] d!").is_err());
        assert!(parse_choice_line_variants("Hello[!], [World!").is_err());
        assert!(parse_choice_line_variants("Hello[!], ]World!").is_err());
    }

    #[test]
    fn parsing_choice_line_variants_return_error_if_brackets_are_reversed() {
        assert!(parse_choice_line_variants("Hello][, World!").is_err());
    }

    #[test]
    fn line_with_choice_markers_parses_into_choice_with_correct_level() {
        let line_text = "Hello, world!";

        let text1 = format!("* {}", line_text);
        let (level, choice) = ParsedLine::from_str(&text1).unwrap().choice();

        assert_eq!(level, 1);
        assert_eq!(choice.line, LineData::from_str(line_text).unwrap());

        let text2 = format!("** {}", line_text);
        let (level, choice) = ParsedLine::from_str(&text2).unwrap().choice();

        assert_eq!(level, 2);
        assert_eq!(choice.line, LineData::from_str(line_text).unwrap());
    }

    #[test]
    fn parsing_choice_sets_displayed_and_line() {
        let line_text = "Hello, world!";
        let choice_text = format!("* {}", line_text);

        let (_, choice) = parse_choice(&choice_text).unwrap().unwrap().choice();

        assert_eq!(&choice.displayed, &choice.line);
    }

    #[test]
    fn choices_are_initialized_with_zero_visits() {
        let line_text = "Hello, world!";
        let choice_text = format!("* {}", line_text);

        let (_, choice) = parse_choice(&choice_text).unwrap().unwrap().choice();

        assert_eq!(choice.num_visited, 0);
    }

    #[test]
    fn asterix_choice_marker_returns_non_sticky_choice() {
        let (_, choice) = ParsedLine::from_str("* Non-sticky choice")
            .unwrap()
            .choice();
        assert!(!choice.is_sticky);
    }

    #[test]
    fn plus_choice_marker_returns_sticky_choice() {
        let (_, choice) = ParsedLine::from_str("+ Non-sticky choice")
            .unwrap()
            .choice();
        assert!(choice.is_sticky);
    }

    #[test]
    fn mix_of_sticky_and_non_sticky_marker_returns_error() {
        assert!(ParsedLine::from_str("+* Some choice ???").is_err());
        assert!(ParsedLine::from_str("*+ Some choice ???").is_err());
        assert!(ParsedLine::from_str("+++ * Some choice ???").is_err());
        assert!(ParsedLine::from_str("**+Some choice ???").is_err());
    }

    #[test]
    fn choice_markers_require_text() {
        assert!(ParsedLine::from_str("*").is_err());
    }

    #[test]
    fn choices_can_be_parsed_with_conditions() {
        let (_, choice) = ParsedLine::from_str("* {knot_name} Hello, World!").unwrap().choice();
        assert_eq!(choice.conditions.len(), 1);
    }
    
    #[test]
    fn choices_with_conditions_still_require_some_text() {
        assert!(ParsedLine::from_str("* {knot_name}").is_err());
    }
}
