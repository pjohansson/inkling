//! Utilities for parsing of lines.

use crate::line::{LineErrorKind, LineParsingError};

#[derive(Clone, Debug, PartialEq)]
/// Text and embraced parts of a line.
///
/// Lines can be split into pure text and text that is enclosed by '{}' braces, which
/// indicate that the internal content should be processed.
pub enum LinePart<'a> {
    /// Pure text part of the line.
    Text(&'a str),
    /// Text part which was enclosed in '{}' braces.
    Embraced(&'a str),
}

/// Split a line into parts of pure text and text enclosed in curly braces.
pub fn split_line_into_variants<'a>(
    content: &'a str,
) -> Result<Vec<LinePart<'a>>, LineParsingError> {
    let mut index = 0;

    let mut parts = Vec::new();

    while let Some(mut i) = content.get(index..).and_then(|s| s.find('{')) {
        i += index;

        if let Some(head) = content.get(index..i) {
            if !head.is_empty() {
                parts.push(LinePart::Text(head));
            }
        }

        let tail = content.get(i..).unwrap();
        let enclosed_content = get_enclosed_content(tail)?;

        parts.push(LinePart::Embraced(enclosed_content));

        index = i + enclosed_content.len() + 2;
    }

    if let Some(tail) = content.get(index..) {
        if tail.contains('}') {
            return Err(LineParsingError {
                kind: LineErrorKind::UnmatchedBraces,
                line: content.to_string(),
            });
        }

        if !tail.is_empty() {
            parts.push(LinePart::Text(tail));
        }
    }

    Ok(parts)
}

/// Get the content enclosed in the first '{}' pair of the line.
fn get_enclosed_content(content: &str) -> Result<&str, LineParsingError> {
    let internal_content = content.get(1..).unwrap();
    let index_closing = find_closing_brace(internal_content)?;

    Ok(internal_content.get(..index_closing).unwrap())
}

/// Find the first right brace '}' that closes the content.
fn find_closing_brace(content: &str) -> Result<usize, LineParsingError> {
    let mut brace_level = 0;

    for (i, c) in content.chars().enumerate() {
        match c {
            '}' if brace_level == 0 => {
                return Ok(i);
            }
            '}' => {
                brace_level -= 1;
            }
            '{' => {
                brace_level += 1;
            }
            _ => (),
        }
    }

    Err(LineParsingError {
        kind: LineErrorKind::UnmatchedBraces,
        line: content.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enclosed_content_from_string_with_single_pair_of_braces_is_the_string() {
        assert_eq!(
            get_enclosed_content("{Hello, World!}").unwrap(),
            "Hello, World!"
        );
    }

    #[test]
    fn enclosed_content_from_string_with_internal_braces_includes_them_too() {
        assert_eq!(
            get_enclosed_content("{Hello, {World}!}").unwrap(),
            "Hello, {World}!"
        );
    }

    #[test]
    fn enclosed_content_from_just_opening_brace_returns_error() {
        assert!(get_enclosed_content("{").is_err());
    }

    #[test]
    fn finding_closing_brace_returns_distance_from_start() {
        assert_eq!(find_closing_brace("}").unwrap(), 0);
        assert_eq!(find_closing_brace("123}").unwrap(), 3);
        assert_eq!(find_closing_brace("   }").unwrap(), 3);
    }

    #[test]
    fn finding_closing_brace_skips_over_internal_brace_pairs() {
        assert_eq!(find_closing_brace("{}}").unwrap(), 2);
        assert_eq!(find_closing_brace("{{}}}").unwrap(), 4);
    }

    #[test]
    fn finding_closing_brace_ignores_braces_after_first_closing() {
        assert_eq!(find_closing_brace("}}").unwrap(), 0);
        assert_eq!(find_closing_brace("}{}").unwrap(), 0);
    }

    #[test]
    fn finding_no_closing_brace_returns_unmatched_braces_error() {
        match find_closing_brace("") {
            Err(LineParsingError {
                kind: LineErrorKind::UnmatchedBraces,
                ..
            }) => (),
            other => panic!(
                "expected `LineErrorKind::UnmatchedBraces` error but got {:?}",
                other
            ),
        }

        match find_closing_brace("{}") {
            Err(LineParsingError {
                kind: LineErrorKind::UnmatchedBraces,
                ..
            }) => (),
            other => panic!(
                "expected `LineErrorKind::UnmatchedBraces` error but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn split_string_on_simple_text_line_gives_single_text_item() {
        let parts = split_line_into_variants("Hello, World!").unwrap();
        assert_eq!(&parts, &[LinePart::Text("Hello, World!")]);
    }

    #[test]
    fn split_string_into_parts_where_curly_braces_are_found() {
        let parts = split_line_into_variants("Hello, {World}!").unwrap();

        assert_eq!(parts[0], LinePart::Text("Hello, "));
        assert_eq!(parts[1], LinePart::Embraced("World"));
        assert_eq!(parts[2], LinePart::Text("!"));
    }

    #[test]
    fn multiple_brace_variants_can_exist_in_the_same_level() {
        let parts = split_line_into_variants("{Hello}, {World}!").unwrap();

        assert_eq!(parts[0], LinePart::Embraced("Hello"));
        assert_eq!(parts[1], LinePart::Text(", "));
        assert_eq!(parts[2], LinePart::Embraced("World"));
        assert_eq!(parts[3], LinePart::Text("!"));
    }

    #[test]
    fn nested_braces_give_string_with_the_braces_intact() {
        let parts = split_line_into_variants("{Hello, {World}!}").unwrap();

        assert_eq!(&parts, &[LinePart::Embraced("Hello, {World}!")]);
    }

    #[test]
    fn unmatched_left_and_right_braces_give_error() {
        assert!(split_line_into_variants("Hello, World!}").is_err());
        assert!(split_line_into_variants("{Hello, World!").is_err());
        assert!(split_line_into_variants("{Hello}, {World!").is_err());
    }
}
