//! Parse choices as marked up `ParsedLineKind::Choice` objects.

use crate::{
    consts::{CHOICE_MARKER, STICKY_CHOICE_MARKER},
    error::{LineErrorKind, LineParsingError},
    line::{
        parse::{
            parse_choice_conditions, parse_internal_line, parse_markers_and_text,
            split_at_divert_marker,
        },
        Content, InternalChoice, InternalChoiceBuilder, InternalLine, ParsedLineKind,
    },
};

/// Parse a `ParsedLineKind::Choice` from a line if the line represents a choice.
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

/// Parse the content of an `InternalChoice` from a line.
///
/// The line should not contain the markers used to determine whether a line of content
/// represents a choice. It should only contain the part of the line which represents
/// the choice text.
fn parse_choice_data(content: &str) -> Result<InternalChoice, LineParsingError> {
    let mut buffer = content.to_string();
    let choice_conditions = parse_choice_conditions(&mut buffer)?;

    let (selection_text_line, display_text_line) = parse_choice_line_variants(&buffer)?;

    let (without_divert, _) = split_at_divert_marker(&selection_text_line);
    let selection_text = parse_internal_line(without_divert)?;

    let is_fallback = is_choice_fallback(&selection_text);

    let display_text = match parse_internal_line(&display_text_line) {
        Err(LineParsingError {
            kind: LineErrorKind::EmptyDivert,
            ..
        }) if is_fallback => {
            let (without_divert, _) = split_at_divert_marker(&display_text_line);
            parse_internal_line(without_divert)
        }
        result => result,
    }?;

    let mut builder = InternalChoiceBuilder::from_line(display_text);

    builder.set_conditions(&choice_conditions);
    builder.set_is_fallback(is_fallback);
    builder.set_selection_text(selection_text);

    Ok(builder.build())
}

/// Check whether a choice line is a fallback.
///
/// The condition for a fallback choice is that it has no displayed text for the user.
fn is_choice_fallback(
    selection_text: &InternalLine,
) -> bool {
    selection_text
        .chunk
        .items
        .iter()
        .all(|item| item == &Content::Empty)
}

/// Split choice markers from a line and determine whether it is sticky.
///
/// If markers are present, ensure that the line does not have both sticky and non-sticky markers.
/// Return the number of markers along with whether the choice was sticky and the remaining line.
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

/// Check whether the input marker appears before the line text content.
fn marker_exists_before_text(line: &str, marker: char) -> bool {
    line.find(|c: char| !(c.is_whitespace() || c == CHOICE_MARKER || c == STICKY_CHOICE_MARKER))
        .map(|i| line.get(..i).unwrap())
        .unwrap_or(line)
        .contains(marker)
}

/// Return `selection_text` and `display_text` strings from a line.
///
/// These are demarcated by `[]` brackets. Content before the bracket is both selection
/// and display text. Content inside the bracket is only for the selection and content
/// after the bracket only for display.
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

            let selection_text = format!("{}{}", head, inside);
            let display_text = format!("{}{}", head, tail);

            Ok((selection_text, display_text))
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

    impl InternalChoice {
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
        let comparison = parse_internal_line("Choice line").unwrap();

        assert_eq!(*choice.selection_text.borrow(), comparison);
        assert_eq!(choice.display_text, comparison);
    }

    #[test]
    fn choices_can_be_parsed_with_alternatives_in_selection_text() {
        let choice = parse_choice_data("Hi! {One|Two}").unwrap();
        assert_eq!(
            *choice.selection_text.borrow(),
            parse_internal_line("Hi! {One|Two}").unwrap(),
        );
    }

    #[test]
    fn braces_with_backslash_are_not_conditions() {
        let choice = parse_choice_data("\\{One|Two}").unwrap();
        assert_eq!(
            *choice.selection_text.borrow(),
            parse_internal_line("{One|Two}").unwrap(),
        );
    }

    #[test]
    fn alternatives_can_be_within_brackets() {
        let choice = parse_choice_data("[{One|Two}]").unwrap();
        assert_eq!(
            *choice.selection_text.borrow(),
            parse_internal_line("{One|Two}").unwrap(),
        );
    }

    #[test]
    fn choice_with_variants_set_selection_and_display_text_separately() {
        let choice = parse_choice_data("Selection[] plus display").unwrap();

        assert_eq!(
            *choice.selection_text.borrow(),
            parse_internal_line("Selection").unwrap()
        );
        assert_eq!(
            choice.display_text,
            parse_internal_line("Selection plus display").unwrap()
        );

        let choice = parse_choice_data("[Separate selection]And display").unwrap();

        assert_eq!(
            *choice.selection_text.borrow(),
            parse_internal_line("Separate selection").unwrap()
        );
        assert_eq!(
            choice.display_text,
            parse_internal_line("And display").unwrap()
        );
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
    fn choices_without_displayed_text_can_have_regular_text() {
        let choice =  parse_choice_data("[]").unwrap();

        assert!(choice.is_fallback);

        assert_eq!(
            choice.display_text,
            parse_internal_line("").unwrap()
        );

        let choice =  parse_choice_data("[] Some text").unwrap();

        assert!(choice.is_fallback);

        assert_eq!(
            choice.display_text,
            parse_internal_line(" Some text").unwrap()
        );
    }

    #[test]
    fn choices_can_be_parsed_with_conditions() {
        let choice = parse_choice_data("{knot_name} Hello, World!").unwrap();
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
