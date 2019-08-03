//! Parsing of story content.
//!
//! While [individual lines][crate::line::parse] and the [nested tree node][crate::node::parse]
//! structure of stitches are parsed elsewhere this module takes individual lines and groups
//! them into knots and stitches. The content is then parsed using the aforementioned modules
//! to create the final story structure.

use crate::{
    consts::{
        KNOT_MARKER, LINE_COMMENT_MARKER, ROOT_KNOT_NAME, STITCH_MARKER, TODO_COMMENT_MARKER,
    },
    error::{KnotError, KnotNameError, ParseError},
    knot::{parse_stitch_from_lines, read_knot_name, read_stitch_name, Knot, KnotSet, Stitch},
};

use std::collections::HashMap;

/// Parse an input string into a set of knots.
///
/// Creates `Stitch`es and their node tree of branching content. Returns the knot collection
/// and the name of the first knot (the story root) in the set.
pub fn read_knots_from_string(content: &str) -> Result<(String, KnotSet), ParseError> {
    let all_lines = content.lines().collect::<Vec<_>>();
    let content_lines = remove_empty_and_comment_lines(all_lines);
    let knot_line_sets = divide_lines_at_marker(content_lines, KNOT_MARKER);

    if knot_line_sets.is_empty() {
        return Err(ParseError::Empty);
    }

    let knots = knot_line_sets
        .into_iter()
        .enumerate()
        .map(|(knot_index, lines)| get_knot_from_lines(lines, knot_index))
        .collect::<Result<Vec<_>, _>>()?;

    let (root_knot_name, _) = knots.first().ok_or(ParseError::Empty)?;

    Ok((root_knot_name.to_string(), knots.into_iter().collect()))
}

/// Parse a single `Knot` from a set of lines.
///
/// Creates `Stitch`es and their node tree of branching content. Returns the knot and its name.
fn get_knot_from_lines(
    mut lines: Vec<&str>,
    knot_index: usize,
) -> Result<(String, Knot), KnotError> {
    let knot_name = get_knot_name(&mut lines, knot_index)?;

    let tags = get_knot_tags(&mut lines);
    let knot_stitch_sets = divide_lines_at_marker(lines, STITCH_MARKER);

    let (default_stitch, stitches) = knot_stitch_sets
        .into_iter()
        .enumerate()
        .map(|(stitch_index, lines)| get_stitch_from_lines(lines, stitch_index, &knot_name))
        .collect::<Result<Vec<_>, _>>()
        .and_then(get_default_stitch_and_hash_map_tuple)?;

    Ok((
        knot_name,
        Knot {
            default_stitch,
            stitches,
            tags,
        },
    ))
}

/// Parse knot tags from lines until the first line with content.
///
/// The lines which contain tags are split off of the input list.
fn get_knot_tags(lines: &mut Vec<&str>) -> Vec<String> {
    if let Some(i) = lines
        .iter()
        .map(|line| line.trim_start())
        .position(|line| !(line.is_empty() || line.starts_with('#')))
    {
        lines
            .drain(..i)
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| line.trim_start_matches("#").trim_start().to_string())
            .collect()
    } else {
        Vec::new()
    }
}

/// Parse a single `Stitch` from a set of lines.
///
/// If a stitch name is found, return it too. This should be found for all stitches except
/// possibly the first in a set, since we split the knot line content where the names are found.
fn get_stitch_from_lines(
    mut lines: Vec<&str>,
    stitch_index: usize,
    knot_name: &str,
) -> Result<(String, Stitch), KnotError> {
    let stitch_name =
        get_stitch_name(&mut lines).map(|name| get_stitch_identifier(name, stitch_index))?;

    let content = parse_stitch_from_lines(&lines, knot_name, &stitch_name)?;

    Ok((stitch_name, content))
}

/// Collect stitches in a map and return along with the root stitch name.
fn get_default_stitch_and_hash_map_tuple(
    stitches: Vec<(String, Stitch)>,
) -> Result<(String, HashMap<String, Stitch>), KnotError> {
    let (default_name, _) = stitches.first().ok_or(KnotError::Empty)?;

    Ok((default_name.clone(), stitches.into_iter().collect()))
}

/// Read knot name from the first line in a set.
///
/// If the name was present, remove that line from the vector and return the name.
/// If it was not present and the knot index is 0, return the
/// [default knot name][crate::consts::ROOT_KNOT_NAME].
fn get_knot_name(lines: &mut Vec<&str>, knot_index: usize) -> Result<String, KnotError> {
    let name_line = lines.first().ok_or(KnotError::Empty)?;

    match (knot_index, read_knot_name(name_line)) {
        (_, Ok(name)) => {
            lines.remove(0);
            Ok(name)
        }
        (
            0,
            Err(KnotError::InvalidName {
                kind: KnotNameError::NoNamePresent,
                ..
            }),
        ) => Ok(ROOT_KNOT_NAME.to_string()),
        (_, Err(err)) => Err(err),
    }
}

/// Read stitch name from the first line in a set.
///
/// If the name was present, remove that line from the vector and return the name.
/// Otherwise return `None`.
fn get_stitch_name(lines: &mut Vec<&str>) -> Result<Option<String>, KnotError> {
    let name_line = lines.first().ok_or(KnotError::Empty)?;

    match read_stitch_name(name_line) {
        Ok(name) => {
            lines.remove(0);
            Ok(Some(name))
        }
        Err(KnotError::InvalidName {
            kind: KnotNameError::NoNamePresent,
            ..
        }) => Ok(None),
        Err(err) => Err(err),
    }
}

/// Get a verified name for a stitch.
///
/// Stitches are name spaced under their parent knot. If the given stitch has no read name
/// but is the first content in the knot, it gets the [default name][crate::consts::ROOT_KNOT_NAME].
fn get_stitch_identifier(name: Option<String>, stitch_index: usize) -> String {
    match (stitch_index, name) {
        (0, None) => ROOT_KNOT_NAME.to_string(),
        (_, Some(name)) => format!("{}", name),
        _ => unreachable!(
            "No stitch name was present after dividing the set of lines into groups where \
             the first line of each group is the stitch name: this is a contradiction which \
             should not be possible."
        ),
    }
}

/// Split a set of lines where they start with a marker.
fn divide_lines_at_marker<'a>(mut content: Vec<&'a str>, marker: &str) -> Vec<Vec<&'a str>> {
    let mut buffer = Vec::new();

    while let Some(i) = content
        .iter()
        .rposition(|line| line.trim_start().starts_with(marker))
    {
        buffer.push(content.split_off(i));
    }

    if !content.is_empty() {
        buffer.push(content);
    }

    buffer.into_iter().rev().collect()
}

/// Filter empty and comment lines from a set.
///
/// Should at some point be removed since we ultimately want to return errors from parsing
/// lines along with their original line numbers, which are thrown away by filtering some
/// of them.
fn remove_empty_and_comment_lines(content: Vec<&str>) -> Vec<&str> {
    content
        .into_iter()
        .enumerate()
        .inspect(|(i, line)| {
            if line.starts_with(TODO_COMMENT_MARKER) {
                eprintln!("{} (line {})", &line, i + 1);
            }
        })
        .map(|(_, line)| line)
        .filter(|line| {
            !(line.starts_with(LINE_COMMENT_MARKER) || line.starts_with(TODO_COMMENT_MARKER))
        })
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            if let Some(i) = line.find("//") {
                line.get(..i).unwrap()
            } else {
                line
            }
        })
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::knot::Address;

    #[test]
    fn read_knots_from_string_works_for_single_nameless_knot() {
        let content = "\
First line.
Second line.
";

        let (head, knots) = read_knots_from_string(content).unwrap();

        assert_eq!(head, ROOT_KNOT_NAME);
        assert_eq!(knots.len(), 1);
        assert!(knots.contains_key(&head));
    }

    #[test]
    fn read_knots_from_string_works_for_single_named_knot() {
        let content = "\
== head ==
First line.
Second line.
";

        let (head, knots) = read_knots_from_string(content).unwrap();

        assert_eq!(head, "head");
        assert_eq!(knots.len(), 1);
        assert!(knots.contains_key(&head));
    }

    #[test]
    fn divide_into_knots_splits_given_lines_at_knot_markers() {
        let content = vec![
            "== Knot one ",
            "Line 1",
            "Line 2",
            "",
            "=== Knot two ===",
            "Line 3",
            "",
        ];

        let knot_lines = divide_lines_at_marker(content.clone(), KNOT_MARKER);

        assert_eq!(knot_lines[0][..], content[0..4]);
        assert_eq!(knot_lines[1][..], content[4..]);
    }

    #[test]
    fn divide_into_knots_adds_content_from_nameless_knots_first() {
        let content = vec!["Line 1", "Line 2", "== Knot one ", "Line 3"];

        let knot_lines = divide_lines_at_marker(content.clone(), KNOT_MARKER);

        assert_eq!(knot_lines[0][..], content[0..2]);
        assert_eq!(knot_lines[1][..], content[2..]);
    }

    #[test]
    fn divide_into_stitches_splits_lines_at_markers() {
        let content = vec![
            "Line 1",
            "= Stitch one ",
            "Line 2",
            "Line 3",
            "",
            "= Stitch two",
            "Line 4",
            "",
        ];

        let knot_lines = divide_lines_at_marker(content.clone(), STITCH_MARKER);

        assert_eq!(knot_lines[0][..], content[0..1]);
        assert_eq!(knot_lines[1][..], content[1..5]);
        assert_eq!(knot_lines[2][..], content[5..]);
    }

    #[test]
    fn empty_lines_and_comment_lines_are_removed_by_initial_processing() {
        let content = vec![
            "Good line",
            "// Comment line is remove",
            "",        // removed
            "       ", // removed
            "TODO: As is todo comments",
            "TODO but not without a colon!",
        ];

        let lines = remove_empty_and_comment_lines(content.clone());
        assert_eq!(&lines, &[content[0].clone(), content[5].clone()]);
    }

    #[test]
    fn initial_processing_splits_off_line_comments() {
        let content = vec![
            "Line before comment marker // Removed part",
            "Line with no comment marker",
        ];

        let lines = remove_empty_and_comment_lines(content.clone());
        assert_eq!(lines[0], "Line before comment marker ");
        assert_eq!(lines[1], "Line with no comment marker");
    }

    #[test]
    fn parsing_knot_from_lines_gets_name() {
        let content = vec!["== Knot_name ==", "Line 1", "Line 2"];

        let (name, _) = get_knot_from_lines(content, 0).unwrap();
        assert_eq!(&name, "Knot_name");
    }

    #[test]
    fn parsing_knot_from_lines_without_stitches_sets_content_in_default_named_stitch() {
        let content = vec!["== Knot_name ==", "Line 1", "Line 2"];

        let (_, knot) = get_knot_from_lines(content, 0).unwrap();

        assert_eq!(&knot.default_stitch, ROOT_KNOT_NAME);
        assert_eq!(
            knot.stitches.get(ROOT_KNOT_NAME).unwrap().root.items.len(),
            2
        );
    }

    #[test]
    fn parsing_a_stitch_gets_name_if_present_else_default_root_name_if_index_is_zero() {
        let (name, _) = get_stitch_from_lines(vec!["= stitch_name =", "Line 1"], 0, "").unwrap();
        assert_eq!(name, "stitch_name".to_string());

        let (name, _) = get_stitch_from_lines(vec!["Line 1"], 0, "").unwrap();
        assert_eq!(name, ROOT_KNOT_NAME);
    }

    #[test]
    fn parsing_stitch_from_lines_sets_address_in_root_node() {
        let (_, stitch) = get_stitch_from_lines(vec!["= cinema", "Line 1"], 0, "tripoli").unwrap();

        assert_eq!(
            stitch.root.address,
            Address::from_parts_unchecked("tripoli", Some("cinema"))
        );

        let (_, stitch) = get_stitch_from_lines(vec!["Line 1"], 0, "tripoli").unwrap();

        assert_eq!(
            stitch.root.address,
            Address::from_parts_unchecked("tripoli", None)
        );
    }

    #[test]
    fn parsing_a_stitch_gets_all_content_regardless_of_whether_name_is_present() {
        let (_, content) = get_stitch_from_lines(vec!["= stitch_name =", "Line 1"], 0, "").unwrap();
        assert_eq!(content.root.items.len(), 1);

        let (_, content) = get_stitch_from_lines(vec!["Line 1"], 0, "").unwrap();
        assert_eq!(content.root.items.len(), 1);
    }

    #[test]
    fn parsing_a_knot_from_lines_sets_stitches_in_hash_map() {
        let lines = vec!["== knot_name", "= stitch_one", "= stitch_two"];
        let (_, knot) = get_knot_from_lines(lines, 0).unwrap();

        assert_eq!(knot.stitches.len(), 2);
        assert!(knot.stitches.get("stitch_one").is_some());
        assert!(knot.stitches.get("stitch_two").is_some());
    }

    #[test]
    fn knot_with_root_content_gets_default_knot_as_first_stitch() {
        let lines = vec![
            "== knot_name",
            "Line 1",
            "= stitch_one",
            "Line 2",
            "= stitch_two",
        ];

        let (_, knot) = get_knot_from_lines(lines, 0).unwrap();
        assert_eq!(&knot.default_stitch, ROOT_KNOT_NAME);
    }

    #[test]
    fn knot_with_no_root_content_gets_default_knot_as_first_stitch() {
        let lines = vec!["== knot_name", "= stitch_one", "Line 1", "= stitch_two"];

        let (_, knot) = get_knot_from_lines(lines, 0).unwrap();
        assert_eq!(&knot.default_stitch, "stitch_one");
    }

    #[test]
    fn knot_parses_tags_from_name_until_first_line_without_octothorpe() {
        let lines = vec!["== knot_name", "# Tag one", "# Tag two", "Line 1"];

        let (_, knot) = get_knot_from_lines(lines, 0).unwrap();
        assert_eq!(&knot.tags, &["Tag one".to_string(), "Tag two".to_string()]);
    }

    #[test]
    fn knot_tags_ignore_empty_lines() {
        let lines = vec!["== knot_name", "", "# Tag one", "", "# Tag two", "Line 1"];

        let (_, knot) = get_knot_from_lines(lines, 0).unwrap();
        assert_eq!(&knot.tags, &["Tag one".to_string(), "Tag two".to_string()]);
    }

    #[test]
    fn if_no_tags_are_set_the_tags_are_empty() {
        let lines = vec!["== knot_name", "Line 1"];

        let (_, knot) = get_knot_from_lines(lines, 0).unwrap();
        assert!(knot.tags.is_empty());
    }

    #[test]
    fn tags_do_not_disturb_remaining_content() {
        let lines_with_tags = vec!["== knot_name", "# Tag one", "# Tag two", "", "Line 1"];
        let lines_without_tags = vec!["== knot_name", "Line 1"];

        let (_, knot_tags) = get_knot_from_lines(lines_with_tags, 0).unwrap();
        let (_, knot_no_tags) = get_knot_from_lines(lines_without_tags, 0).unwrap();

        assert_eq!(
            format!("{:?}", knot_tags.stitches),
            format!("{:?}", knot_no_tags.stitches)
        );
    }
}
