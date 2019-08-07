//! Utilities for parsing of lines.

use crate::error::{LineError, LineErrorKind};

use std::{iter::once, ops::Range};

#[derive(Clone, Debug, PartialEq)]
/// Text and embraced parts of a line.
///
/// Lines can be split into pure text and text that is enclosed by braces, which
/// indicate that the internal content should be processed.
pub enum LinePart<'a> {
    /// Pure text part of the line.
    Text(&'a str),
    /// Text part which was enclosed in braces.
    Embraced(&'a str),
}

/// Return line split at a separator, ignoring separators inside curly braces.
///
/// Wrapper around `split_line_at_separator` with curly braces as open and close.
pub fn split_line_at_separator_braces<'a>(
    content: &'a str,
    separator: &str,
    max_splits: Option<usize>,
) -> Result<Vec<&'a str>, LineError> {
    split_line_at_separator(content, separator, max_splits, '{', '}')
}

/// Return line split at a separator, ignoring separators inside parenthesis.
///
/// Wrapper around `split_line_at_separator` with parenthesis as open and close.
pub fn split_line_at_separator_parenthesis<'a>(
    content: &'a str,
    separator: &str,
    max_splits: Option<usize>,
) -> Result<Vec<&'a str>, LineError> {
    split_line_at_separator(content, separator, max_splits, '(', ')')
}

#[allow(dead_code)]
/// Return line split at a separator, ignoring separators inside double quotes.
///
/// Wrapper around `split_line_at_separator` with double quotes as open and close.
pub fn split_line_at_separator_quotes<'a>(
    content: &'a str,
    separator: &str,
    max_splits: Option<usize>,
) -> Result<Vec<&'a str>, LineError> {
    split_line_at_separator(content, separator, max_splits, '"', '"')
}

/// Return line split at a separator.
///
/// # Notes
/// *   Should work for strings with multibyte characters, since we search for braces
///     based on their byte indices, not char index position.
/// *   Will not work if the separator itself includes curly '{}' braces.
/// *   Separators can be escaped with a leading backslash '\' (has to be removed later).
fn split_line_at_separator<'a>(
    content: &'a str,
    separator: &str,
    max_splits: Option<usize>,
    open: char,
    close: char,
) -> Result<Vec<&'a str>, LineError> {
    let outside_brace_ranges = get_brace_level_zero_ranges(content, open, close)?;

    let separator_indices = get_separator_indices(content, &outside_brace_ranges, separator);

    let separator_size = separator.as_bytes().len();
    let num_bytes = content.as_bytes().len();

    let iter_start = once(0).chain(separator_indices.iter().map(|&i| i + separator_size));

    let iter_end = separator_indices
        .iter()
        .take(max_splits.unwrap_or(separator_indices.len()))
        .chain(once(&num_bytes));

    Ok(iter_start
        .zip(iter_end)
        .map(|(start, &end)| content.get(start..end).unwrap())
        .collect())
}

/// Split a line into parts of pure text and text enclosed in braces.
///
/// Wrapper around `split_line_into_groups` with curly braces as open and close.
pub fn split_line_into_groups_braces<'a>(content: &'a str) -> Result<Vec<LinePart<'a>>, LineError> {
    split_line_into_groups(content, '{', '}')
}

#[allow(dead_code)]
/// Split a line into parts of pure text and text enclosed in parenthesis.
///
/// Wrapper around `split_line_into_groups` with parenthesis as open and close.
pub fn split_line_into_groups_parenthesis<'a>(
    content: &'a str,
) -> Result<Vec<LinePart<'a>>, LineError> {
    split_line_into_groups(content, '(', ')')
}

/// Split a line into parts of pure text and text enclosed between a pair of characters.
fn split_line_into_groups<'a>(
    content: &'a str,
    open: char,
    close: char,
) -> Result<Vec<LinePart<'a>>, LineError> {
    let outside_brace_ranges = get_brace_level_zero_ranges(content, open, close)?;
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

/// Get a list of all separator indices that are not within any of the given `Range`s.
fn get_separator_indices(
    content: &str,
    outside_brace_ranges: &[Range<usize>],
    separator: &str,
) -> Vec<usize> {
    let backslash_preceeding_indices = content
        .match_indices('\\')
        .map(|(i, _)| i + 1)
        .collect::<Vec<_>>();

    content
        .match_indices(separator)
        .map(|(i, _)| i)
        .filter(|i| outside_brace_ranges.iter().any(|range| range.contains(i)))
        .filter(|i| !backslash_preceeding_indices.contains(i))
        .collect::<Vec<_>>()
}

/// Find the `Range`s of bytes in a string which are not enclosed by curly braces.
///
/// Any given variant of opening and closing characters can be used.
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
/// *   Opening and closing characters must be single-byte characters.
fn get_brace_level_zero_ranges(
    content: &str,
    open: char,
    close: char,
) -> Result<Vec<Range<usize>>, LineError> {
    let brace_levels = get_brace_level_of_line(content, open, close)?;
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

/// Map every byte in a string to how many braces are nested for it.
///
/// Any given variant of opening and closing characters can be used.
///
/// If the opening and closing characters are identical the nesting level toggles between
/// zero and one every time the character is encountered.
///
/// # Notes
/// *   Braces can be preceeded with backslashes ('\') in which case they do not
///     count as nesting braces.
/// *   Opening and closing characters must be single-byte characters.
///
/// # Example
/// ```ignore
/// assert_eq!(
///     &get_brace_level_of_line("0{}{{2}}{}0", '{', '}').unwrap(),
///     &[0, 1, 0, 1, 2, 2, 1, 0, 1, 0, 0]
/// );
/// ```
fn get_brace_level_of_line(content: &str, open: char, close: char) -> Result<Vec<u8>, LineError> {
    content
        .bytes()
        .scan((None, 0), |(prev, brace_level), byte| {
            if byte == open as u8 && prev.map(|c| c != b'\\').unwrap_or(true) {
                *brace_level += 1;
            } else if byte == close as u8 && prev.map(|c| c != b'\\').unwrap_or(true) {
                if *brace_level > 0 {
                    *brace_level -= 1;
                } else {
                    return Some(Err(LineError::from_kind(
                        content,
                        LineErrorKind::UnmatchedBraces,
                    )));
                }
            }

            prev.replace(byte);

            if open == close {
                *brace_level = *brace_level % 2;
            }

            Some(Ok(*brace_level))
        })
        .collect::<Result<Vec<_>, _>>()
        .and_then(|brace_levels| {
            if brace_levels.last().map(|&v| v == 0).unwrap_or(true) {
                Ok(brace_levels)
            } else {
                Err(LineError::from_kind(
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
    fn split_empty_string_at_separator_returns_empty_string() {
        assert_eq!(
            split_line_at_separator("", "|", None, '{', '}').unwrap(),
            &[""]
        );
    }

    #[test]
    fn split_empty_string_with_separators_return_multiple_empty_strings() {
        assert_eq!(
            split_line_at_separator("||", "|", None, '{', '}').unwrap(),
            &["", "", ""]
        );
    }

    #[test]
    fn splitting_string_at_separators_returns_content() {
        assert_eq!(
            split_line_at_separator("Hello|World!", "|", None, '{', '}').unwrap(),
            &["Hello", "World!"]
        );
    }

    #[test]
    fn maximum_number_of_splits_can_be_supplied() {
        assert_eq!(
            split_line_at_separator("One|Two|Three", "|", Some(0), '{', '}').unwrap(),
            &["One|Two|Three"]
        );

        assert_eq!(
            split_line_at_separator("One|Two|Three", "|", Some(1), '{', '}').unwrap(),
            &["One", "Two|Three"]
        );

        assert_eq!(
            split_line_at_separator("One|Two|Three", "|", Some(2), '{', '}').unwrap(),
            &["One", "Two", "Three"]
        );

        assert_eq!(
            split_line_at_separator("One|Two|Three", "|", Some(3), '{', '}').unwrap(),
            &["One", "Two", "Three"]
        );
    }

    #[test]
    fn any_separator_can_be_used() {
        assert_eq!(
            split_line_at_separator("One|Two", "|", None, '{', '}').unwrap(),
            &["One", "Two"]
        );

        assert_eq!(
            split_line_at_separator("One,Two", ",", None, '{', '}').unwrap(),
            &["One", "Two"]
        );

        assert_eq!(
            split_line_at_separator("One$Two", "$", None, '{', '}').unwrap(),
            &["One", "Two"]
        );
    }

    #[test]
    fn separators_are_ignored_with_preceeding_backslash() {
        assert_eq!(
            split_line_at_separator("One \\| Still One | Two", "|", None, '{', '}').unwrap(),
            &["One \\| Still One ", " Two"],
        );
    }

    #[test]
    fn splitting_string_with_separator_inside_curly_braces_returns_one_item() {
        assert_eq!(
            split_line_at_separator("{Hello|World!}", "|", None, '{', '}').unwrap(),
            &["{Hello|World!}"]
        );
    }

    #[test]
    fn splitting_string_with_multichar_separator_works() {
        assert_eq!(
            split_line_at_separator("Hello$!$World!", "$!$", None, '{', '}').unwrap(),
            &["Hello", "World!"]
        );
    }

    #[test]
    fn splitting_string_with_multibyte_separator_works() {
        assert_eq!(
            split_line_at_separator("Hello택World!", "택", None, '{', '}').unwrap(),
            &["Hello", "World!"]
        );

        assert_eq!(
            split_line_at_separator("He택l{lo택Wo}rl택d!", "택", None, '{', '}').unwrap(),
            &["He", "l{lo택Wo}rl", "d!"]
        );
    }

    #[test]
    fn splitting_string_with_mixed_braces_and_separators_return_correct_items() {
        assert_eq!(
            split_line_at_separator("Hello, {World|!}|Again!", "|", None, '{', '}').unwrap(),
            &["Hello, {World|!}", "Again!"]
        );
    }

    #[test]
    fn splitting_string_with_unmatched_braces_returns_error() {
        assert!(split_line_at_separator("}Hello, World!", "|", None, '{', '}').is_err());
        assert!(split_line_at_separator("{Hello, World!", "|", None, '{', '}').is_err());
        assert!(split_line_at_separator("Hello, {World{}!", "|", None, '{', '}').is_err());
    }

    #[test]
    fn split_string_on_simple_text_line_gives_single_text_item() {
        let parts = split_line_into_groups("Hello, World!", '{', '}').unwrap();
        assert_eq!(&parts, &[LinePart::Text("Hello, World!")]);
    }

    #[test]
    fn split_string_into_parts_where_curly_braces_are_found() {
        let parts = split_line_into_groups("Hello, {World}!", '{', '}').unwrap();

        assert_eq!(parts[0], LinePart::Text("Hello, "));
        assert_eq!(parts[1], LinePart::Embraced("World"));
        assert_eq!(parts[2], LinePart::Text("!"));
    }

    #[test]
    fn empty_strings_are_split_into_zero_parts() {
        assert!(split_line_into_groups("", '{', '}').unwrap().is_empty());
    }

    #[test]
    fn beginning_with_braced_content_adds_it_as_embraced() {
        let parts = split_line_into_groups("{Hello}, World!", '{', '}').unwrap();
        assert_eq!(&parts[0], &LinePart::Embraced("Hello"));
    }

    #[test]
    fn ending_with_braced_content_adds_it_as_embraced() {
        let parts = split_line_into_groups("Hello, {World!}", '{', '}').unwrap();
        assert_eq!(&parts[1], &LinePart::Embraced("World!"));
    }

    #[test]
    fn multiple_brace_variants_can_exist_in_the_same_level() {
        let parts = split_line_into_groups("{Hello}, {World}!", '{', '}').unwrap();

        assert_eq!(parts[0], LinePart::Embraced("Hello"));
        assert_eq!(parts[1], LinePart::Text(", "));
        assert_eq!(parts[2], LinePart::Embraced("World"));
        assert_eq!(parts[3], LinePart::Text("!"));
    }

    #[test]
    fn nested_braces_give_string_with_the_braces_intact() {
        let parts = split_line_into_groups("{Hello, {World}!}", '{', '}').unwrap();

        assert_eq!(&parts, &[LinePart::Embraced("Hello, {World}!")]);
    }

    #[test]
    fn multiple_nested_braces_split_correctly() {
        let parts = split_line_into_groups("Hello, {World{!}}", '{', '}').unwrap();

        assert_eq!(&parts[1], &LinePart::Embraced("World{!}"));
    }

    #[test]
    fn unmatched_left_and_right_braces_give_error() {
        assert!(split_line_into_groups("Hello, World!}", '{', '}').is_err());
        assert!(split_line_into_groups("{Hello, World!", '{', '}').is_err());
        assert!(split_line_into_groups("{Hello}, {World!", '{', '}').is_err());
    }

    #[test]
    fn string_with_no_braces_give_single_full_range() {
        assert_eq!(get_brace_level_zero_ranges("", '{', '}').unwrap(), &[]);
        assert_eq!(
            get_brace_level_zero_ranges("Hello, World!", '{', '}').unwrap(),
            &[Range { start: 0, end: 13 }]
        );
    }

    #[test]
    fn string_with_braces_in_the_middle_get_surrounding_ranges() {
        assert_eq!(
            get_brace_level_zero_ranges("Hello,{} World!", '{', '}').unwrap(),
            &[Range { start: 0, end: 6 }, Range { start: 8, end: 15 }]
        );
        assert_eq!(
            get_brace_level_zero_ranges("Hello, {World}!", '{', '}').unwrap(),
            &[Range { start: 0, end: 7 }, Range { start: 14, end: 15 }]
        );
    }

    #[test]
    fn braces_can_be_next_to_each_other_yielding_empty_ranges() {
        assert_eq!(
            get_brace_level_zero_ranges("Hello,{}{}World!", '{', '}').unwrap(),
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
            get_brace_level_zero_ranges("{}Hello, World!{}", '{', '}').unwrap(),
            &[Range { start: 2, end: 15 }]
        );
    }

    #[test]
    fn single_character_before_brace_gives_single_item_range() {
        assert_eq!(
            get_brace_level_zero_ranges("a{}", '{', '}').unwrap(),
            &[Range { start: 0, end: 1 }]
        );
    }

    #[test]
    fn brace_level_counting_works_for_empty_line() {
        assert_eq!(get_brace_level_of_line("", '{', '}').unwrap(), &[]);
    }

    #[test]
    fn brace_level_of_line_with_no_braces_is_zero() {
        assert_eq!(
            get_brace_level_of_line("Hello", '{', '}').unwrap(),
            &[0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn brace_level_counting_works_for_wider_chars() {
        assert_eq!(
            get_brace_level_of_line("김{택}용", '{', '}').unwrap(),
            &[0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0]
        );
    }

    #[test]
    fn single_brace_pair_in_middle_sets_brace_level_one_exclusive_end() {
        assert_eq!(
            get_brace_level_of_line("He{ll}o", '{', '}').unwrap(),
            &[0, 0, 1, 1, 1, 0, 0]
        );
    }

    #[test]
    fn other_open_and_close_characters_can_be_used() {
        assert_eq!(
            get_brace_level_of_line("He(ll)o", '(', ')').unwrap(),
            &[0, 0, 1, 1, 1, 0, 0]
        );
    }

    #[test]
    fn nested_brace_pairs_sets_higher_brace_levels() {
        assert_eq!(
            get_brace_level_of_line("He{l{l}}o", '{', '}').unwrap(),
            &[0, 0, 1, 1, 2, 2, 1, 0, 0]
        );
    }

    #[test]
    fn verify_get_brace_level_of_line_doctest_example() {
        assert_eq!(
            &get_brace_level_of_line("0{}{{2}}{}0", '{', '}').unwrap(),
            &[0, 1, 0, 1, 2, 2, 1, 0, 1, 0, 0]
        );
    }

    #[test]
    fn unmatched_braces_yield_error_from_brace_level_counting() {
        assert!(get_brace_level_of_line("{Hello", '{', '}').is_err());
        assert!(get_brace_level_of_line("}Hello", '{', '}').is_err());
        assert!(get_brace_level_of_line("Hel{{}lo", '{', '}').is_err());
        assert!(get_brace_level_of_line("Hel{}}lo", '{', '}').is_err());
    }

    #[test]
    fn braces_with_leading_backslashes_do_not_increase_or_decrease_the_level() {
        assert_eq!(
            &get_brace_level_of_line("Hello, World!", '{', '}').unwrap(),
            &vec![0; 13],
        );

        assert_eq!(
            &get_brace_level_of_line("\\{Hello, World!", '{', '}').unwrap(),
            &vec![0; 15],
        );

        assert_eq!(
            &get_brace_level_of_line("\\}Hello, World!", '{', '}').unwrap(),
            &vec![0; 15],
        );

        assert_eq!(
            &get_brace_level_of_line("Hello\\{, \\}World!", '{', '}').unwrap(),
            &vec![0; 17],
        );
    }

    #[test]
    fn same_open_and_close_character_simply_toggles_levels_on_and_off() {
        assert_eq!(
            &get_brace_level_of_line("\"one\"zero\"one\"zero\"one\"", '"', '"').unwrap(),
            &[1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 0]
        );
    }
}
