//! Parse `Alternative` line chunks.

use crate::{
    consts::{CYCLE_MARKER, ONCE_ONLY_MARKER, SEQUENCE_SEPARATOR, SHUFFLE_MARKER},
    error::LineParsingError,
    line::{
        parse::{parse_chunk, split_line_at_separator_braces},
        Alternative, AlternativeBuilder, AlternativeKind,
    },
};

/// Parse an `Alternative` object from a line.
///
/// # Notes
/// *   The line should not have the enclosing '{}' braces that mark line variations.
/// *   Trims the line from the beginning to the first non-whitespace character.
pub fn parse_alternative(content: &str) -> Result<Alternative, LineParsingError> {
    let (tail, kind) = get_alternative_kind_and_cut_marker(content.trim_start());

    let items = split_line_at_separator_braces(tail, SEQUENCE_SEPARATOR, None)?
        .into_iter()
        .map(|text| parse_chunk(text))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AlternativeBuilder::from_kind(kind)
        .with_items(items)
        .build())
}

/// Determine the alternating sequence kind and return the string without the marker.
fn get_alternative_kind_and_cut_marker(content: &str) -> (&str, AlternativeKind) {
    match get_sequence_kind(content) {
        AlternativeKind::Sequence => (content, AlternativeKind::Sequence),
        kind => (content.get(1..).unwrap(), kind),
    }
}

/// Determine the kind of alternating sequence a string represents.
fn get_sequence_kind(content: &str) -> AlternativeKind {
    if content.starts_with(CYCLE_MARKER) {
        AlternativeKind::Cycle
    } else if content.starts_with(ONCE_ONLY_MARKER) {
        AlternativeKind::OnceOnly
    } else if content.starts_with(SHUFFLE_MARKER) {
        eprintln!(
            "WARNING: Shuffle sequences are not yet implemented. Creating a `Cycle` sequence. \
             (line was: '{}')",
            content
        );

        AlternativeKind::Cycle
    } else {
        AlternativeKind::Sequence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::process::line::tests::{get_processed_alternative, get_processed_chunk};

    #[test]
    fn list_of_strings_separated_by_vertical_lines_are_added_to_set() {
        let text = "One|Two|Three";

        let mut alternative = parse_alternative(text).unwrap();

        assert_eq!(alternative.items.len(), 3);

        assert_eq!(&get_processed_chunk(&mut alternative.items[0]), "One");
        assert_eq!(&get_processed_chunk(&mut alternative.items[1]), "Two");
        assert_eq!(&get_processed_chunk(&mut alternative.items[2]), "Three");
    }

    #[test]
    fn plain_list_of_strings_give_regular_sequence() {
        let text = "One|Two|Three";

        match &parse_alternative(text).unwrap().kind {
            AlternativeKind::Sequence => (),
            kind => panic!("expected `AlternativeKind::Sequence` but got {:?}", kind),
        }
    }

    #[test]
    fn list_of_strings_beginning_with_ampersand_gives_cycle() {
        let text = "&One|Two|Three";

        match &parse_alternative(text).unwrap().kind {
            AlternativeKind::Cycle => (),
            kind => panic!("expected `AlternativeKind::Cycle` but got {:?}", kind),
        }
    }

    #[test]
    fn list_of_strings_beginning_with_exclamation_mark_gives_once_only() {
        let text = "!One|Two|Three";

        match &parse_alternative(text).unwrap().kind {
            AlternativeKind::OnceOnly => (),
            kind => panic!("expected `AlternativeKind::OnceOnly` but got {:?}", kind),
        }
    }

    #[test]
    fn whitespace_is_trimmed_from_the_beginning() {
        let text = " &One|Two|Three";
        let mut alternative = parse_alternative(text).unwrap();

        assert_eq!(&get_processed_alternative(&mut alternative), "One");

        match &alternative.kind {
            AlternativeKind::Cycle => (),
            kind => panic!("expected `AlternativeKind::Cycle` but got {:?}", kind),
        }
    }

    #[test]
    fn empty_strings_do_not_fail() {
        assert!(parse_alternative("").is_ok());
    }
}
