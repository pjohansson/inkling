//! Utilities for parsing of lines.

use crate::line::{LineErrorKind, LineParsingError};

use std::{iter::once, ops::Range};

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

/// Return line split at a separator, ignoring separators inside curly braces.
///
/// # Notes
/// *   Should work for strings with multibyte characters, since we search for braces
///     based on their byte indices, not char index position.
/// *   Will not work if the separator itself includes curly '{}' braces.
pub fn split_line_at_separator<'a>(
    content: &'a str,
    separator: &str,
) -> Result<Vec<&'a str>, LineParsingError> {
    let outside_brace_ranges = get_brace_level_zero_ranges(content)?;
    let separator_indices = content
        .match_indices(separator)
        .map(|(i, _)| i)
        .filter(|i| outside_brace_ranges.iter().any(|range| range.contains(i)))
        .collect::<Vec<_>>();

    let separator_size = separator.as_bytes().len();
    let num_bytes = content.as_bytes().len();

    let iter_start = once(0).chain(separator_indices.iter().map(|&i| i + separator_size));
    let iter_end = separator_indices.iter().chain(once(&num_bytes));

    Ok(iter_start
        .zip(iter_end)
        .map(|(start, &end)| content.get(start..end).unwrap())
        .collect())
}

/// Split a line into parts of pure text and text enclosed in curly braces.
pub fn split_line_into_variants<'a>(
    content: &'a str,
) -> Result<Vec<LinePart<'a>>, LineParsingError> {
    let outside_brace_ranges = get_brace_level_zero_ranges(content)?;
    let num_bytes = content.as_bytes().len();

    let mut iter = outside_brace_ranges.iter().peekable();
    let mut parts: Vec<LinePart> = Vec::new();

    while let Some(&Range { start, end }) = iter.next() {
        if parts.is_empty() && start > 0 {
            let text = content.get(1..start - 1).unwrap();
            parts.push(LinePart::Embraced(text));
        }

        let text = content.get(start..end).unwrap();
        parts.push(LinePart::Text(text));

        if let Some(&Range {
            start: start_next, ..
        }) = iter.peek()
        {
            let text = content.get(end + 1..*start_next - 1).unwrap();
            parts.push(LinePart::Embraced(text));
        } else {
            if let Some(text) = content.get(end + 1..num_bytes - 1) {
                parts.push(LinePart::Embraced(text));
            }
        }
    }

    // If the line is purely embraced content we add that content here
    if parts.is_empty() && num_bytes > 0 {
        if let Some(text) = content.get(1..num_bytes - 1) {
            parts.push(LinePart::Embraced(text));
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

/// Find the `Range`s of bytes in a string which are not enclosed by curly braces.
///
/// Since content withing braces should be kept together we often will not want to split
/// lines in the middle of them. So we need a way to identify where in a string these braces
/// are before operating on it.
///
/// This function returns all the byte ranges in a string which are not enclosed by matching
/// braces.
///
/// # Notes
/// *   Yes, the returned ranges are byte ranges instead of character index ranges.
fn get_brace_level_zero_ranges(content: &str) -> Result<Vec<Range<usize>>, LineParsingError> {
    let brace_levels = get_brace_level_of_line(content)?;
    let num_bytes = brace_levels.len();

    let mut iter = brace_levels.into_iter().enumerate().peekable();

    let mut start_range = Vec::new();
    let mut end_range = Vec::new();

    while let Some((i, level)) = iter.next() {
        if i == 0 && level == 0 {
            start_range.push(i);
        }

        if let Some((_, next_level)) = iter.peek() {
            match (level, next_level) {
                (0, 1) => end_range.push(i + 1),
                (1, 0) => start_range.push(i + 2),
                _ => (),
            }
        }
    }

    if end_range.len() < start_range.len() && *start_range.last().unwrap() < num_bytes {
        end_range.push(num_bytes);
    }

    Ok(start_range
        .into_iter()
        .zip(end_range.into_iter())
        .map(|(start, end)| Range { start, end })
        .collect())
}

/// Map every byte in a string to how many curly braces are nested for it.
///
/// # Example
/// ```ignore
/// assert_eq!(
///     &get_brace_level_of_line("0{}{{2}}{}0").unwrap(),
///     &[0, 1, 0, 1, 2, 2, 1, 0, 1, 0, 0]
/// );
/// ```
fn get_brace_level_of_line(content: &str) -> Result<Vec<u8>, LineParsingError> {
    content
        .bytes()
        .scan(0, |brace_level, b| {
            if b == b'{' {
                *brace_level += 1;
            } else if b == b'}' {
                if *brace_level > 0 {
                    *brace_level -= 1;
                } else {
                    return Some(Err(LineParsingError::from_kind(
                        content,
                        LineErrorKind::UnmatchedBraces,
                    )));
                }
            }

            Some(Ok(*brace_level))
        })
        .collect::<Result<Vec<_>, _>>()
        .and_then(|brace_levels| {
            if brace_levels.last().map(|&v| v == 0).unwrap_or(true) {
                Ok(brace_levels)
            } else {
                Err(LineParsingError::from_kind(
                    content,
                    LineErrorKind::UnmatchedBraces,
                ))
            }
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
    fn split_empty_string_at_separator_returns_empty_string() {
        assert_eq!(split_line_at_separator("", "|").unwrap(), &[""]);
    }

    #[test]
    fn split_empty_string_with_separators_return_multiple_empty_strings() {
        assert_eq!(split_line_at_separator("||", "|").unwrap(), &["", "", ""]);
    }

    #[test]
    fn splitting_string_at_separators_returns_content() {
        assert_eq!(
            split_line_at_separator("Hello|World!", "|").unwrap(),
            &["Hello", "World!"]
        );
    }

    #[test]
    fn any_separator_can_be_used() {
        assert_eq!(
            split_line_at_separator("One|Two", "|").unwrap(),
            &["One", "Two"]
        );

        assert_eq!(
            split_line_at_separator("One,Two", ",").unwrap(),
            &["One", "Two"]
        );

        assert_eq!(
            split_line_at_separator("One$Two", "$").unwrap(),
            &["One", "Two"]
        );
    }

    #[test]
    fn splitting_string_with_separator_inside_curly_braces_returns_one_item() {
        assert_eq!(
            split_line_at_separator("{Hello|World!}", "|").unwrap(),
            &["{Hello|World!}"]
        );
    }

    #[test]
    fn splitting_string_with_multichar_separator_works() {
        assert_eq!(
            split_line_at_separator("Hello$!$World!", "$!$").unwrap(),
            &["Hello", "World!"]
        );
    }

    #[test]
    fn splitting_string_with_multibyte_separator_works() {
        assert_eq!(
            split_line_at_separator("Hello택World!", "택").unwrap(),
            &["Hello", "World!"]
        );

        assert_eq!(
            split_line_at_separator("He택l{lo택Wo}rl택d!", "택").unwrap(),
            &["He", "l{lo택Wo}rl", "d!"]
        );
    }

    #[test]
    fn splitting_string_with_mixed_braces_and_separators_return_correct_items() {
        assert_eq!(
            split_line_at_separator("Hello, {World|!}|Again!", "|").unwrap(),
            &["Hello, {World|!}", "Again!"]
        );
    }

    #[test]
    fn splitting_string_with_unmatched_braces_returns_error() {
        assert!(split_line_at_separator("}Hello, World!", "|").is_err());
        assert!(split_line_at_separator("{Hello, World!", "|").is_err());
        assert!(split_line_at_separator("Hello, {World{}!", "|").is_err());
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
    fn empty_strings_are_split_into_zero_parts() {
        assert!(split_line_into_variants("").unwrap().is_empty());
    }

    #[test]
    fn beginning_with_braced_content_adds_it_as_embraced() {
        let parts = split_line_into_variants("{Hello}, World!").unwrap();
        assert_eq!(&parts[0], &LinePart::Embraced("Hello"));
    }

    #[test]
    fn ending_with_braced_content_adds_it_as_embraced() {
        let parts = split_line_into_variants("Hello, {World!}").unwrap();
        assert_eq!(&parts[1], &LinePart::Embraced("World!"));
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
    fn multiple_nested_braces_split_correctly() {
        let parts = split_line_into_variants("Hello, {World{!}}").unwrap();

        assert_eq!(&parts[1], &LinePart::Embraced("World{!}"));
    }

    #[test]
    fn unmatched_left_and_right_braces_give_error() {
        assert!(split_line_into_variants("Hello, World!}").is_err());
        assert!(split_line_into_variants("{Hello, World!").is_err());
        assert!(split_line_into_variants("{Hello}, {World!").is_err());
    }

    #[test]
    fn string_with_no_braces_give_single_full_range() {
        assert_eq!(get_brace_level_zero_ranges("").unwrap(), &[]);
        assert_eq!(
            get_brace_level_zero_ranges("Hello, World!").unwrap(),
            &[Range { start: 0, end: 13 }]
        );
    }

    #[test]
    fn string_with_braces_in_the_middle_get_surrounding_ranges() {
        assert_eq!(
            get_brace_level_zero_ranges("Hello,{} World!").unwrap(),
            &[Range { start: 0, end: 6 }, Range { start: 8, end: 15 }]
        );
        assert_eq!(
            get_brace_level_zero_ranges("Hello, {World}!").unwrap(),
            &[Range { start: 0, end: 7 }, Range { start: 14, end: 15 }]
        );
    }

    #[test]
    fn braces_can_be_next_to_each_other_yielding_empty_ranges() {
        assert_eq!(
            get_brace_level_zero_ranges("Hello,{}{}World!").unwrap(),
            &[
                Range { start: 0, end: 6 },
                Range { start: 8, end: 8 },
                Range { start: 10, end: 16 }
            ]
        );
    }

    #[test]
    fn braces_can_be_at_beginning_or_end_and_will_not_be_included_in_ranges() {
        assert_eq!(
            get_brace_level_zero_ranges("{}Hello, World!{}").unwrap(),
            &[Range { start: 2, end: 15 }]
        );
    }

    #[test]
    fn single_character_before_brace_gives_single_item_range() {
        assert_eq!(
            get_brace_level_zero_ranges("a{}").unwrap(),
            &[Range { start: 0, end: 1 }]
        );
    }

    #[test]
    fn brace_level_counting_works_for_empty_line() {
        assert_eq!(get_brace_level_of_line("").unwrap(), &[]);
    }

    #[test]
    fn brace_level_of_line_with_no_braces_is_zero() {
        assert_eq!(get_brace_level_of_line("Hello").unwrap(), &[0, 0, 0, 0, 0]);
    }

    #[test]
    fn brace_level_counting_works_for_wider_chars() {
        assert_eq!(
            get_brace_level_of_line("김{택}용").unwrap(),
            &[0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0]
        );
    }

    #[test]
    fn single_brace_pair_in_middle_sets_brace_level_one_exclusive_end() {
        assert_eq!(
            get_brace_level_of_line("He{ll}o").unwrap(),
            &[0, 0, 1, 1, 1, 0, 0]
        );
    }

    #[test]
    fn nested_brace_pairs_sets_higher_brace_levels() {
        assert_eq!(
            get_brace_level_of_line("He{l{l}}o").unwrap(),
            &[0, 0, 1, 1, 2, 2, 1, 0, 0]
        );
    }

    #[test]
    fn verify_get_brace_level_of_line_doctest_example() {
        assert_eq!(
            &get_brace_level_of_line("0{}{{2}}{}0").unwrap(),
            &[0, 1, 0, 1, 2, 2, 1, 0, 1, 0, 0]
        );
    }

    #[test]
    fn unmatched_braces_yield_error_from_brace_level_counting() {
        assert!(get_brace_level_of_line("{Hello").is_err());
        assert!(get_brace_level_of_line("}Hello").is_err());
        assert!(get_brace_level_of_line("Hel{{}lo").is_err());
        assert!(get_brace_level_of_line("Hel{}}lo").is_err());
    }
}
