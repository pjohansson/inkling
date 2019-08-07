//! Parsing of story content.
//!
//! While [individual lines][crate::line::parse] and the [nested tree node][crate::node::parse]
//! structure of stitches are parsed elsewhere this module takes individual lines and groups
//! them into knots and stitches. The content is then parsed using the aforementioned modules
//! to create the final story structure.

use crate::{
    consts::{
        CONST_MARKER, EXTERNAL_FUNCTION_MARKER, INCLUDE_MARKER, KNOT_MARKER, LINE_COMMENT_MARKER,
        ROOT_KNOT_NAME, STITCH_MARKER, TAG_MARKER, TODO_COMMENT_MARKER, VARIABLE_MARKER,
    },
    error::{KnotError, KnotNameError, LineError, LineErrorKind, ParseError},
    knot::{parse_stitch_from_lines, read_knot_name, read_stitch_name, Knot, KnotSet, Stitch},
    line::{parse_variable, Variable},
    story::VariableSet,
    utils::MetaData,
};

use std::collections::HashMap;

/// Read an Ink story from a string and return knots along with the metadata.
pub fn read_story_content_from_string(
    content: &str,
) -> Result<(KnotSet, VariableSet, Vec<String>), ParseError> {
    let all_lines = content
        .lines()
        .enumerate()
        .map(|(i, line)| {
            (
                line,
                MetaData {
                    line_index: i as u32,
                },
            )
        })
        .collect::<Vec<_>>();

    let content_lines = remove_empty_and_comment_lines(all_lines);

    let (prelude_lines, knot_lines) = split_lines_into_prelude_and_knots(&content_lines);
    let (metadata_lines, root_content) = split_prelude_into_metadata_and_text(&prelude_lines);

    let root_knot = parse_root_knot_from_lines(root_content)?;
    let mut knots = parse_knots_from_lines(knot_lines)?;

    let tags = parse_global_tags(&metadata_lines);
    let variables = parse_global_variables(&metadata_lines)?;

    knots.insert(ROOT_KNOT_NAME.to_string(), root_knot);

    Ok((knots, variables, tags))
}

/// Parse all knots from a set of lines.
fn parse_knots_from_lines(lines: Vec<(&str, MetaData)>) -> Result<KnotSet, ParseError> {
    let knot_line_sets = divide_lines_at_marker(lines, KNOT_MARKER);

    let knots = knot_line_sets
        .into_iter()
        .map(|lines| get_knot_from_lines(lines))
        .collect::<Result<KnotSet, _>>()?;

    Ok(knots.into_iter().collect())
}

/// Parse the root knot from a set of lines.
fn parse_root_knot_from_lines(lines: Vec<(&str, MetaData)>) -> Result<Knot, KnotError> {
    let stitches = get_stitches_from_lines(lines, ROOT_KNOT_NAME)?
        .into_iter()
        .collect();

    Ok(Knot {
        default_stitch: ROOT_KNOT_NAME.to_string(),
        stitches,
        tags: Vec::new(),
        meta_data: MetaData { line_index: 0 },
    })
}

/// Parse a single `Knot` from a set of lines.
///
/// Creates `Stitch`es and their node tree of branching content. Returns the knot and its name.
fn get_knot_from_lines(lines: Vec<(&str, MetaData)>) -> Result<(String, Knot), KnotError> {
    let (head, mut tail) = lines
        .split_first()
        .map(|(head, tail)| (head, tail.to_vec()))
        .ok_or(KnotError::EmptyKnot)?;

    let (head, meta_data) = head;

    let knot_name = read_knot_name(head)?;
    let tags = get_knot_tags(&mut tail);

    let (default_stitch, stitches) = get_stitches_from_lines(tail, &knot_name)
        .and_then(get_default_stitch_and_hash_map_tuple)?;

    Ok((
        knot_name,
        Knot {
            default_stitch,
            stitches,
            tags,
            meta_data: meta_data.clone(),
        },
    ))
}

/// Parse all stitches from a set of lines.
fn get_stitches_from_lines(
    lines: Vec<(&str, MetaData)>,
    knot_name: &str,
) -> Result<Vec<(String, Stitch)>, KnotError> {
    let knot_stitch_sets = divide_lines_at_marker(lines, STITCH_MARKER);

    knot_stitch_sets
        .into_iter()
        .enumerate()
        .filter(|(_, lines)| !lines.is_empty())
        .map(|(stitch_index, lines)| get_stitch_from_lines(lines, stitch_index, knot_name))
        .collect::<Result<Vec<_>, _>>()
}

/// Parse knot tags from lines until the first line with content.
///
/// The lines which contain tags are split off of the input list.
fn get_knot_tags(lines: &mut Vec<(&str, MetaData)>) -> Vec<String> {
    if let Some(i) = lines
        .iter()
        .map(|(line, _)| line.trim_start())
        .position(|line| !(line.is_empty() || line.starts_with('#')))
    {
        lines
            .drain(..i)
            .map(|(line, _)| line.trim())
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
    mut lines: Vec<(&str, MetaData)>,
    stitch_index: usize,
    knot_name: &str,
) -> Result<(String, Stitch), KnotError> {
    let (stitch_name, meta_data) = get_stitch_name_and_meta_data(&mut lines)
        .map(|(name, meta_data)| (get_stitch_identifier(name, stitch_index), meta_data))?;

    let content = parse_stitch_from_lines(&lines, knot_name, &stitch_name, meta_data)?;

    Ok((stitch_name, content))
}

/// Collect stitches in a map and return along with the root stitch name.
fn get_default_stitch_and_hash_map_tuple(
    stitches: Vec<(String, Stitch)>,
) -> Result<(String, HashMap<String, Stitch>), KnotError> {
    let (default_name, _) = stitches.first().ok_or(KnotError::EmptyKnot)?;

    Ok((default_name.clone(), stitches.into_iter().collect()))
}

/// Read stitch name and metadata from the first line in a set.
///
/// If the name was present, remove that line from the vector and return the name.
/// Otherwise return `None`.
fn get_stitch_name_and_meta_data(
    lines: &mut Vec<(&str, MetaData)>,
) -> Result<(Option<String>, MetaData), KnotError> {
    let (name_line, meta_data) = lines.first().cloned().ok_or(KnotError::EmptyStitch)?;

    match read_stitch_name(name_line) {
        Ok(name) => {
            lines.remove(0);

            Ok((Some(name), meta_data))
        }
        Err(KnotError::InvalidName {
            kind: KnotNameError::NoNamePresent,
            ..
        }) => Ok((None, meta_data)),
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
fn divide_lines_at_marker<'a>(
    mut content: Vec<(&'a str, MetaData)>,
    marker: &str,
) -> Vec<Vec<(&'a str, MetaData)>> {
    let mut buffer = Vec::new();

    while let Some(i) = content
        .iter()
        .rposition(|(line, _)| line.trim_start().starts_with(marker))
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
fn remove_empty_and_comment_lines(content: Vec<(&str, MetaData)>) -> Vec<(&str, MetaData)> {
    content
        .into_iter()
        .inspect(|(line, meta_data)| {
            if line.starts_with(TODO_COMMENT_MARKER) {
                eprintln!("{} (line {})", &line, meta_data.line_index + 1);
            }
        })
        .filter(|(line, _)| {
            !(line.starts_with(LINE_COMMENT_MARKER) || line.starts_with(TODO_COMMENT_MARKER))
        })
        .filter(|(line, _)| !line.trim().is_empty())
        .map(|(line, meta_data)| {
            if let Some(i) = line.find("//") {
                (line.get(..i).unwrap(), meta_data)
            } else {
                (line, meta_data)
            }
        })
        .collect()
}

/// Split given list of lines into a prelude and knot content.
///
/// The prelude contains metadata and the root knot, which the story will start from.
fn split_lines_into_prelude_and_knots<'a>(
    lines: &[(&'a str, MetaData)],
) -> (Vec<(&'a str, MetaData)>, Vec<(&'a str, MetaData)>) {
    if let Some(i) = lines
        .iter()
        .position(|(line, _)| line.trim_start().starts_with(KNOT_MARKER))
    {
        let (prelude, knots) = lines.split_at(i);
        (prelude.to_vec(), knots.to_vec())
    } else {
        (lines.to_vec(), Vec::new())
    }
}

/// Split prelude content into metadata and root text content.
fn split_prelude_into_metadata_and_text<'a>(
    lines: &[(&'a str, MetaData)],
) -> (Vec<(&'a str, MetaData)>, Vec<(&'a str, MetaData)>) {
    // Add spaces after all keywords (except line comment) to search for whole words.
    let metadata_keywords = &[
        format!("{} ", CONST_MARKER),
        format!("{} ", EXTERNAL_FUNCTION_MARKER),
        format!("{} ", INCLUDE_MARKER),
        format!("{} ", VARIABLE_MARKER),
        format!("{} ", TODO_COMMENT_MARKER),
        format!("{}", LINE_COMMENT_MARKER),
    ];

    const METADATA_CHARS: &[char] = &[TAG_MARKER];

    if let Some(i) = lines
        .iter()
        .map(|(line, _)| line.trim_start())
        .position(|line| {
            metadata_keywords.iter().all(|key| !line.starts_with(key))
                && METADATA_CHARS.iter().all(|&c| !line.starts_with(c))
                && !line.is_empty()
        })
    {
        let (metadata, text) = lines.split_at(i);
        (metadata.to_vec(), text.to_vec())
    } else {
        (lines.to_vec(), Vec::new())
    }
}

/// Parse global tags from a set of metadata lines in the prelude.
fn parse_global_tags(lines: &[(&str, MetaData)]) -> Vec<String> {
    lines
        .iter()
        .map(|(line, _)| line.trim())
        .filter(|line| line.starts_with(TAG_MARKER))
        .map(|line| line.get(1..).unwrap().trim().to_string())
        .collect()
}

/// Parse global variables from a set of metadata lines in the prelude.
fn parse_global_variables(
    lines: &[(&str, MetaData)],
) -> Result<HashMap<String, Variable>, LineError> {
    lines
        .iter()
        .map(|(line, meta_data)| (line.trim(), meta_data))
        .filter(|(line, _)| line.starts_with(VARIABLE_MARKER))
        .map(|(line, meta_data)| {
            parse_variable_with_name(line).map_err(|kind| LineError {
                line: line.to_string(),
                kind,
                meta_data: meta_data.clone(),
            })
        })
        .collect()
}

/// Parse a single variable line into the variable name and initial value.
///
/// Variable lines are on the form `VAR variable_name = initial_value`.
fn parse_variable_with_name(line: &str) -> Result<(String, Variable), LineErrorKind> {
    line.find('=')
        .ok_or_else(|| unimplemented!())
        .and_then(|i| {
            let start = VARIABLE_MARKER.len();
            let variable_name = line.get(start..i).unwrap().trim().to_string();

            if variable_name.is_empty() {
                Err(LineErrorKind::NoVariableName)
            } else {
                Ok((i, variable_name))
            }
        })
        .and_then(|(i, variable_name)| {
            let variable_value = parse_variable(line.get(i + 1..).unwrap().trim())?;
            Ok((variable_name, variable_value))
        })
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::knot::Address;

    pub fn read_knots_from_string(content: &str) -> Result<KnotSet, ParseError> {
        let lines = content
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(i, line)| (line, MetaData::from(i)))
            .collect();

        parse_knots_from_lines(lines)
    }

    fn enumerate<'a>(lines: &[&'a str]) -> Vec<(&'a str, MetaData)> {
        lines
            .into_iter()
            .map(|line| *line)
            .enumerate()
            .map(|(i, line)| (line, MetaData::from(i)))
            .collect()
    }

    fn denumerate<'a, T>(lines: Vec<(&'a str, T)>) -> Vec<&'a str> {
        lines.into_iter().map(|(line, _)| line).collect()
    }

    #[test]
    fn split_lines_into_knots_and_prelude() {
        let lines = &[
            "Prelude content ",
            "comes before ",
            "the first named knot.",
            "",
            "=== here ===",
            "Line one.",
        ];

        let (prelude, knots) = split_lines_into_prelude_and_knots(&enumerate(lines));

        assert_eq!(
            &denumerate(prelude),
            &[
                "Prelude content ",
                "comes before ",
                "the first named knot.",
                ""
            ]
        );
        assert_eq!(&denumerate(knots), &["=== here ===", "Line one."]);
    }

    #[test]
    fn prelude_can_be_further_split_into_metadata_and_prelude_text() {
        let lines = &[
            "# All prelude content",
            "",
            "# comes before",
            "The first regular string.",
        ];

        let (metadata, text) = split_prelude_into_metadata_and_text(&enumerate(lines));

        assert_eq!(
            &denumerate(metadata),
            &["# All prelude content", "", "# comes before"]
        );
        assert_eq!(&denumerate(text), &["The first regular string."]);
    }

    #[test]
    fn metadata_stops_when_it_does_not_start_with_variable_include_or_tag() {
        let lines = &[
            "# Tag",
            "VAR variable",
            "CONST constant variable",
            "INCLUDE include",
            "// line comment",
            "TODO: comment",
            "Regular line.",
        ];

        let (metadata, text) = split_prelude_into_metadata_and_text(&enumerate(lines));

        assert_eq!(metadata.len(), 6);
        assert_eq!(&denumerate(text), &["Regular line."]);
    }

    #[test]
    fn parse_global_tags_from_metadata() {
        let lines = &[
            "# Tag",
            "VAR variable",
            "# Tag two ",
            "// line comment",
            "TODO: comment",
        ];

        assert_eq!(&parse_global_tags(&enumerate(lines)), &["Tag", "Tag two"]);
    }

    #[test]
    fn parse_variables_from_metadata() {
        let lines = &[
            "# Tag",
            "VAR float = 1.0",
            "# Tag two ",
            "VAR string = \"two words\"",
        ];

        let variables = parse_global_variables(&enumerate(lines)).unwrap();

        assert_eq!(variables.len(), 2);
        assert_eq!(variables.get("float").unwrap(), &Variable::Float(1.0));
        assert_eq!(
            variables.get("string").unwrap(),
            &Variable::String("two words".to_string())
        );
    }

    #[test]
    fn regular_lines_can_start_with_variable_divert_or_text() {
        let lines = &["# Tag", "Regular line."];

        let (_, text) = split_prelude_into_metadata_and_text(&enumerate(lines));

        assert_eq!(&denumerate(text), &["Regular line."]);

        let lines_divert = &["# Tag", "-> divert"];

        let (_, divert) = split_prelude_into_metadata_and_text(&enumerate(lines_divert));

        assert_eq!(&denumerate(divert), &["-> divert"]);

        let lines_variable = &["# Tag", "{variable}"];

        let (_, variable) = split_prelude_into_metadata_and_text(&enumerate(lines_variable));

        assert_eq!(&denumerate(variable), &["{variable}"]);
    }

    #[test]
    fn read_knots_from_string_reads_several_present_knots() {
        let content = "\
== first ==
First line.

== second ==
First line.

== third ==
First line.
";

        let knots = read_knots_from_string(content).unwrap();

        assert_eq!(knots.len(), 3);

        assert!(knots.contains_key("first"));
        assert!(knots.contains_key("second"));
        assert!(knots.contains_key("third"));
    }

    #[test]
    fn read_knots_from_string_requires_named_knots() {
        let content = "\
First line.
Second line.
";

        assert!(read_knots_from_string(content).is_err());
    }

    #[test]
    fn divide_into_knots_splits_given_lines_at_knot_markers() {
        let content = enumerate(&[
            "== Knot one ",
            "Line 1",
            "Line 2",
            "",
            "=== Knot two ===",
            "Line 3",
            "",
        ]);

        let knot_lines = divide_lines_at_marker(content.clone(), KNOT_MARKER);

        assert_eq!(knot_lines[0][..], content[0..4]);
        assert_eq!(knot_lines[1][..], content[4..]);
    }

    #[test]
    fn divide_into_knots_adds_content_from_nameless_knots_first() {
        let content = enumerate(&["Line 1", "Line 2", "== Knot one ", "Line 3"]);

        let knot_lines = divide_lines_at_marker(content.clone(), KNOT_MARKER);

        assert_eq!(knot_lines[0][..], content[0..2]);
        assert_eq!(knot_lines[1][..], content[2..]);
    }

    #[test]
    fn divide_into_stitches_splits_lines_at_markers() {
        let content = enumerate(&[
            "Line 1",
            "= Stitch one ",
            "Line 2",
            "Line 3",
            "",
            "= Stitch two",
            "Line 4",
            "",
        ]);

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

        let lines = remove_empty_and_comment_lines(enumerate(&content));
        assert_eq!(
            &denumerate(lines),
            &[content[0].clone(), content[5].clone()]
        );
    }

    #[test]
    fn initial_processing_splits_off_line_comments() {
        let content = vec![
            "Line before comment marker // Removed part",
            "Line with no comment marker",
        ];

        let lines = remove_empty_and_comment_lines(enumerate(&content));
        assert_eq!(lines[0].0, "Line before comment marker ");
        assert_eq!(lines[1].0, "Line with no comment marker");
    }

    #[test]
    fn parsing_knot_from_lines_gets_name() {
        let content = enumerate(&["== Knot_name ==", "Line 1", "Line 2"]);

        let (name, _) = get_knot_from_lines(content).unwrap();
        assert_eq!(&name, "Knot_name");
    }

    #[test]
    fn parsing_knot_from_lines_without_stitches_sets_content_in_default_named_stitch() {
        let content = enumerate(&["== Knot_name ==", "Line 1", "Line 2"]);

        let (_, knot) = get_knot_from_lines(content).unwrap();

        assert_eq!(&knot.default_stitch, ROOT_KNOT_NAME);
        assert_eq!(
            knot.stitches.get(ROOT_KNOT_NAME).unwrap().root.items.len(),
            2
        );
    }

    #[test]
    fn parsing_a_stitch_gets_name_if_present_else_default_root_name_if_index_is_zero() {
        let (name, _) =
            get_stitch_from_lines(enumerate(&["= stitch_name =", "Line 1"]), 0, "").unwrap();
        assert_eq!(name, "stitch_name".to_string());

        let (name, _) = get_stitch_from_lines(enumerate(&["Line 1"]), 0, "").unwrap();
        assert_eq!(name, ROOT_KNOT_NAME);
    }

    #[test]
    fn parsing_stitch_from_lines_sets_address_in_root_node() {
        let (_, stitch) =
            get_stitch_from_lines(enumerate(&["= cinema", "Line 1"]), 0, "tripoli").unwrap();

        assert_eq!(
            stitch.root.address,
            Address::from_parts_unchecked("tripoli", Some("cinema"))
        );

        let (_, stitch) = get_stitch_from_lines(enumerate(&["Line 1"]), 0, "tripoli").unwrap();

        assert_eq!(
            stitch.root.address,
            Address::from_parts_unchecked("tripoli", None)
        );
    }

    #[test]
    fn parsing_a_stitch_gets_all_content_regardless_of_whether_name_is_present() {
        let (_, content) =
            get_stitch_from_lines(enumerate(&["= stitch_name =", "Line 1"]), 0, "").unwrap();
        assert_eq!(content.root.items.len(), 1);

        let (_, content) = get_stitch_from_lines(enumerate(&["Line 1"]), 0, "").unwrap();
        assert_eq!(content.root.items.len(), 1);
    }

    #[test]
    fn parsing_a_knot_from_lines_sets_stitches_in_hash_map() {
        let lines = enumerate(&[
            "== knot_name",
            "= stitch_one",
            "Line one",
            "= stitch_two",
            "Line two",
        ]);

        let (_, knot) = get_knot_from_lines(lines).unwrap();

        assert_eq!(knot.stitches.len(), 2);
        assert!(knot.stitches.get("stitch_one").is_some());
        assert!(knot.stitches.get("stitch_two").is_some());
    }

    #[test]
    fn knot_with_root_content_gets_default_knot_as_first_stitch() {
        let lines = enumerate(&[
            "== knot_name",
            "Line 1",
            "= stitch_one",
            "Line 2",
            "= stitch_two",
            "Line 3",
        ]);

        let (_, knot) = get_knot_from_lines(lines).unwrap();
        assert_eq!(&knot.default_stitch, ROOT_KNOT_NAME);
    }

    #[test]
    fn root_knot_parses_stitch_without_a_name() {
        let lines = enumerate(&["Line 1", "Line 2"]);

        let root = parse_root_knot_from_lines(lines.clone()).unwrap();

        let comparison =
            parse_stitch_from_lines(&lines, ROOT_KNOT_NAME, ROOT_KNOT_NAME, ().into()).unwrap();

        assert_eq!(
            format!("{:?}", root.stitches.get(ROOT_KNOT_NAME).unwrap()),
            format!("{:?}", comparison)
        );
    }

    #[test]
    fn root_knot_may_have_stitches() {
        let lines = enumerate(&["Line 1", "= Stitch", "Line 2"]);

        let root = parse_root_knot_from_lines(lines).unwrap();

        assert_eq!(root.stitches.len(), 2);
    }

    #[test]
    fn knot_with_no_root_content_gets_default_knot_as_first_stitch() {
        let lines = enumerate(&[
            "== knot_name",
            "= stitch_one",
            "Line 1",
            "= stitch_two",
            "Line 2",
        ]);

        let (_, knot) = get_knot_from_lines(lines).unwrap();
        assert_eq!(&knot.default_stitch, "stitch_one");
    }

    #[test]
    fn knot_parses_tags_from_name_until_first_line_without_octothorpe() {
        let lines = enumerate(&["== knot_name", "# Tag one", "# Tag two", "Line 1"]);

        let (_, knot) = get_knot_from_lines(lines).unwrap();
        assert_eq!(&knot.tags, &["Tag one".to_string(), "Tag two".to_string()]);
    }

    #[test]
    fn knot_tags_ignore_empty_lines() {
        let lines = enumerate(&["== knot_name", "", "# Tag one", "", "# Tag two", "Line 1"]);

        let (_, knot) = get_knot_from_lines(lines).unwrap();
        assert_eq!(&knot.tags, &["Tag one".to_string(), "Tag two".to_string()]);
    }

    #[test]
    fn if_no_tags_are_set_the_tags_are_empty() {
        let lines = enumerate(&["== knot_name", "Line 1"]);

        let (_, knot) = get_knot_from_lines(lines).unwrap();
        assert!(knot.tags.is_empty());
    }

    #[test]
    fn tags_do_not_disturb_remaining_content() {
        let lines_with_tags = enumerate(&["== knot_name", "# Tag one", "# Tag two", "", "Line 1"]);
        let lines_without_tags = vec![
            ("== knot_name", MetaData::from(0)),
            ("Line 1", MetaData::from(4)),
        ];

        let (_, knot_tags) = get_knot_from_lines(lines_with_tags).unwrap();
        let (_, knot_no_tags) = get_knot_from_lines(lines_without_tags).unwrap();

        assert_eq!(
            format!("{:?}", knot_tags.stitches),
            format!("{:?}", knot_no_tags.stitches)
        );
    }

    #[test]
    fn reading_story_data_gets_unordered_variables_in_prelude() {
        let content = "
# Random tag
VAR counter = 0
# Random tag
// Line comment
VAR hazardous = true

-> introduction
";

        let (_, variables, _) = read_story_content_from_string(content).unwrap();

        assert_eq!(variables.len(), 2);
        assert!(variables.contains_key("counter"));
        assert!(variables.contains_key("hazardous"));
    }

    #[test]
    fn variables_after_first_line_of_text_are_ignored() {
        let content = "
VAR counter = 0

-> introduction
VAR hazardous = true
";

        let (_, variables, _) = read_story_content_from_string(content).unwrap();

        assert_eq!(variables.len(), 1);
        assert!(variables.contains_key("counter"));
    }

    #[test]
    fn no_variables_give_empty_set() {
        let content = "
// Just a line comment!
-> introduction
";

        let (_, variables, _) = read_story_content_from_string(content).unwrap();

        assert_eq!(variables.len(), 0);
    }

    #[test]
    fn reading_story_data_gets_all_global_tags_in_prelude() {
        let content = "
# title: test
VAR counter = 0
# rating: hazardous
// Line comment
VAR hazardous = true

-> introduction
";

        let (_, _, tags) = read_story_content_from_string(content).unwrap();

        assert_eq!(
            &tags,
            &["title: test".to_string(), "rating: hazardous".to_string()]
        );
    }

    #[test]
    fn reading_story_data_sets_knot_line_starting_line_indices_including_prelude_content() {
        let content = "\
# title: line_counting
VAR line_count = 0

-> root

== root
One line.

== second
Second line.
";

        let (knots, _, _) = read_story_content_from_string(content).unwrap();

        assert_eq!(knots.get("root").unwrap().meta_data.line_index, 5);
        assert_eq!(knots.get("second").unwrap().meta_data.line_index, 8);
    }
}
