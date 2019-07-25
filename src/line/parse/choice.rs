use crate::{
    consts::{CHOICE_MARKER, STICKY_CHOICE_MARKER},
    line::{
        parse_choice_conditions, parse_line, parse_markers_and_text, split_at_divert_marker,
        Content, FullChoice, FullChoiceBuilder, FullLine, LineErrorKind, LineParsingError,
        ParsedLineKind,
    },
};

pub fn parse_choice(content: &str) -> Result<Option<ParsedLineKind>, LineParsingError> {
    parse_choice_markers_and_text(content)?
        .map(|(level, is_sticky, line)| {
            parse_choice_data(line)
                .map(|mut choice_data| {
                    choice_data.is_sticky = is_sticky;
                    (level, choice_data)
                })
                .map(|(level, choice_data)| ParsedLineKind::Choice { level, choice_data })
        })
        .transpose()
}

fn parse_choice_data(content: &str) -> Result<FullChoice, LineParsingError> {
    let mut buffer = content.to_string();
    let choice_conditions = parse_choice_conditions(&mut buffer).unwrap();

    let (selection_text_line, display_text_line) = parse_choice_line_variants(&buffer)?;

    let (without_divert, _) = split_at_divert_marker(&selection_text_line);
    let selection_text = parse_line(without_divert)?;

    let is_fallback = is_choice_fallback(&selection_text, content)?;

    let display_text = match parse_line(&display_text_line) {
        Err(LineParsingError {
            kind: LineErrorKind::EmptyDivert,
            ..
        }) if is_fallback => {
            let (without_divert, _) = split_at_divert_marker(&display_text_line);
            parse_line(without_divert)
        }
        result => result,
    }?;

    let mut builder = FullChoiceBuilder::from_line(display_text);

    builder.set_conditions(&choice_conditions);
    builder.set_is_fallback(is_fallback);
    builder.set_selection_text(selection_text);

    Ok(builder.build())
}

/// Check whether a choice line is a fallback. The condition for a fallback choice
/// is that it has no displayed text for the user.
///
/// A choice with no displayed text can have no regular text, either. Return an error
/// if it has a separator between the displayed choice and follow up text.
fn is_choice_fallback(
    selection_text: &FullLine,
    original_line: &str,
) -> Result<bool, LineParsingError> {
    let is_fallback = selection_text
        .chunk
        .items
        .iter()
        .all(|item| item == &Content::Empty);

    let choice_has_separator = original_line.find('[').is_some();

    if is_fallback && choice_has_separator {
        Err(LineParsingError {
            kind: LineErrorKind::BlankChoice,
            line: original_line.to_string(),
        })
    } else {
        Ok(is_fallback)
    }
}

/// Split choice markers (sticky or non-sticky) from a line. If they are present, ensure
/// that the line does not have both sticky and non-sticky markers. Return the number
/// of markers along with whether the choice was sticky and the remaining line.
pub fn parse_choice_markers_and_text(
    content: &str,
) -> Result<Option<(u32, bool, &str)>, LineParsingError> {
    let is_sticky = marker_exists_before_text(content, STICKY_CHOICE_MARKER);
    let is_not_sticky = marker_exists_before_text(content, CHOICE_MARKER);

    let marker = match (is_sticky, is_not_sticky) {
        (false, false) => None,
        (true, false) => Some(STICKY_CHOICE_MARKER),
        (false, true) => Some(CHOICE_MARKER),
        (true, true) => {
            return Err(LineParsingError {
                kind: LineErrorKind::StickyAndNonSticky,
                line: content.to_string(),
            });
        }
    };

    marker
        .and_then(|c| parse_markers_and_text(content, c))
        .map(|(level, line)| Ok((level, is_sticky, line)))
        .transpose()
}

fn marker_exists_before_text(line: &str, marker: char) -> bool {
    line.find(|c: char| !(c.is_whitespace() || c == CHOICE_MARKER || c == STICKY_CHOICE_MARKER))
        .map(|i| line.get(..i).unwrap())
        .unwrap_or(line)
        .contains(marker)
}

fn parse_choice_line_variants(line: &str) -> Result<(String, String), LineParsingError> {
    match (line.find('['), line.find(']')) {
        (Some(i), Some(j)) if i < j => {
            // Ensure that we don't have more brackets
            if line.rfind('[').unwrap() != i || line.rfind(']').unwrap() != j {
                return Err(LineParsingError {
                    kind: LineErrorKind::UnmatchedBrackets,
                    line: line.to_string(),
                });
            }

            let head = line.get(..i).unwrap();
            let inside = line.get(i + 1..j).unwrap();
            let tail = line.get(j + 1..).unwrap();

            let displayed = format!("{}{}", head, inside);
            let line = format!("{}{}", head, tail);

            Ok((displayed, line))
        }
        (None, None) => Ok((line.to_string(), line.to_string())),
        _ => Err(LineParsingError {
            kind: LineErrorKind::UnmatchedBrackets,
            line: line.to_string(),
        }),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    impl FullChoice {
        pub fn from_string(line: &str) -> Self {
            parse_choice_data(line).unwrap()
        }
    }

    #[test]
    fn parsing_line_with_no_choice_markers_returns_none() {
        assert!(parse_choice_markers_and_text("Choice").unwrap().is_none());
        assert!(parse_choice_markers_and_text("  Choice  ")
            .unwrap()
            .is_none());
        assert!(parse_choice_markers_and_text("- Choice  ")
            .unwrap()
            .is_none());
    }

    #[test]
    fn parsing_line_with_choice_markers_gets_number_of_markers() {
        let (level, _, _) = parse_choice_markers_and_text("* Choice").unwrap().unwrap();
        assert_eq!(level, 1);

        let (level, _, _) = parse_choice_markers_and_text("** Choice").unwrap().unwrap();
        assert_eq!(level, 2);

        let (level, _, _) = parse_choice_markers_and_text("**** Choice")
            .unwrap()
            .unwrap();
        assert_eq!(level, 4);
    }

    #[test]
    fn number_of_markers_parsing_ignores_whitespace() {
        let (level, _, _) = parse_choice_markers_and_text("  * * *   *     Choice")
            .unwrap()
            .unwrap();
        assert_eq!(level, 4);
    }

    #[test]
    fn sticky_choice_markers_gives_sticky_choices_and_vice_versa() {
        let (_, is_sticky, _) = parse_choice_markers_and_text("* Choice").unwrap().unwrap();
        assert!(!is_sticky);

        let (_, is_sticky, _) = parse_choice_markers_and_text("+ Choice").unwrap().unwrap();
        assert!(is_sticky);
    }

    #[test]
    fn lines_cannot_have_both_sticky_and_non_sticky_markers_in_the_head() {
        assert!(parse_choice_markers_and_text("*+ Choice").is_err());
        assert!(parse_choice_markers_and_text("+* Choice").is_err());
        assert!(parse_choice_markers_and_text(" +++*+ Choice").is_err());
        assert!(parse_choice_markers_and_text("+ Choice *").is_ok());
    }

    #[test]
    fn text_after_choice_markers_is_returned_when_parsing() {
        let (_, _, line) = parse_choice_markers_and_text("* * Choice")
            .unwrap()
            .unwrap();
        assert_eq!(line, "Choice");

        let (_, _, line) = parse_choice_markers_and_text("+++ Choice")
            .unwrap()
            .unwrap();
        assert_eq!(line, "Choice");
    }

    #[test]
    fn simple_lines_parse_into_choices_with_same_display_and_selection_texts() {
        let choice = parse_choice_data("Choice line").unwrap();
        let comparison = parse_line("Choice line").unwrap();

        assert_eq!(choice.selection_text, comparison);
        assert_eq!(choice.display_text, comparison);
    }

    #[test]
    fn choice_with_variants_set_selection_and_display_text_separately() {
        let choice = parse_choice_data("Selection[] plus display").unwrap();

        assert_eq!(choice.selection_text, parse_line("Selection").unwrap());
        assert_eq!(
            choice.display_text,
            parse_line("Selection plus display").unwrap()
        );

        let choice = parse_choice_data("[Separate selection]And display").unwrap();

        assert_eq!(
            choice.selection_text,
            parse_line("Separate selection").unwrap()
        );
        assert_eq!(choice.display_text, parse_line("And display").unwrap());
    }

    #[test]
    fn choice_with_no_selection_text_but_divert_is_fallback() {
        assert!(parse_choice_data("-> world").unwrap().is_fallback);
        assert!(parse_choice_data(" -> world").unwrap().is_fallback);
    }

    #[test]
    fn choice_which_is_fallback_can_have_empty_divert() {
        assert!(parse_choice_data("->").expect("one").is_fallback);
        assert!(parse_choice_data(" -> ").expect("two").is_fallback);
    }

    #[test]
    fn choices_without_displayed_text_cannot_have_regular_text() {
        match parse_choice_data("[]") {
            Err(LineParsingError {
                kind: LineErrorKind::BlankChoice,
                ..
            }) => (),
            other => panic!("expected `LineErrorKind::BlankChoice` but got {:?}", other),
        }

        match parse_choice_data("[] Some text") {
            Err(LineParsingError {
                kind: LineErrorKind::BlankChoice,
                ..
            }) => (),
            other => panic!("expected `LineErrorKind::BlankChoice` but got {:?}", other),
        }
    }

    #[test]
    fn choices_can_be_parsed_with_conditions() {
        let choice = parse_choice_data("* {knot_name} Hello, World!").unwrap();
        assert_eq!(choice.conditions.len(), 1);
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
}
