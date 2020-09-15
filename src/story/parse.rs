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
    error::{
        parse::{
            knot::{KnotError, KnotErrorKind, KnotNameError},
            prelude::{PreludeError, PreludeErrorKind},
            ParseError,
        },
        utils::MetaData,
        ReadError,
    },
    knot::{parse_stitch_from_lines, read_knot_name, read_stitch_name, Knot, KnotSet, Stitch},
    line::parse_variable,
    story::types::{VariableInfo, VariableSet},
};

use std::collections::HashMap;

/// Read an Ink story from a string and return knots along with the metadata.
pub fn read_story_content_from_string(
    content: &str,
) -> Result<(KnotSet, VariableSet, Vec<String>), ReadError> {
    let all_lines = content
        .lines()
        .zip(0..)
        .map(|(line, line_index)| (line, MetaData { line_index }))
        .collect::<Vec<_>>();

    let mut content_lines = remove_empty_and_comment_lines(all_lines);

    let (root_knot, variables, tags, prelude_errors) =
        split_off_and_parse_prelude(&mut content_lines)?;

    let (mut knots, mut knot_errors) = parse_knots_from_lines(content_lines);

    match root_knot {
        Ok(knot) => {
            knots.insert(ROOT_KNOT_NAME.to_string(), knot);
        }
        Err(knot_error) => knot_errors.insert(0, knot_error),
    }

    if knot_errors.is_empty() && prelude_errors.is_empty() {
        Ok((knots, variables, tags))
    } else {
        Err(ParseError {
            knot_errors,
            prelude_errors,
        }
        .into())
    }
}

/// Split off lines until the first named knot then parse its content and root knot.
///
/// After this function has been called, the given set of lines starts at the first named
/// knot.
///
/// Will return an error only if there is no story content in the given list. Prelude errors
/// are collected into a list which is returned in the `Ok` value. Any errors from parsing
/// the root knot is returned in that item. This is all because we want to collect all
/// encountered errors from parsing the story at once, not just the first.
fn split_off_and_parse_prelude(
    lines: &mut Vec<(&str, MetaData)>,
) -> Result<
    (
        Result<Knot, KnotError>,
        VariableSet,
        Vec<String>,
        Vec<PreludeError>,
    ),
    ReadError,
> {
    let prelude_and_root = split_off_prelude_lines(lines);
    let (prelude_lines, root_lines) = split_prelude_into_metadata_and_text(&prelude_and_root);

    let root_meta_data = root_lines
        .first()
        .or(lines.first())
        .or(prelude_lines.last())
        .map(|(_, meta_data)| meta_data.clone())
        .ok_or(ReadError::Empty)?;

    let tags = parse_global_tags(&prelude_lines);
    let (variables, prelude_errors) = parse_global_variables(&prelude_lines);
    let root_knot = parse_root_knot_from_lines(root_lines, root_meta_data);

    Ok((root_knot, variables, tags, prelude_errors))
}

/// Parse all knots from a set of lines and return along with any encountered errors.
fn parse_knots_from_lines(lines: Vec<(&str, MetaData)>) -> (KnotSet, Vec<KnotError>) {
    let knot_line_sets = divide_lines_at_marker(lines, KNOT_MARKER);

    let mut knots = HashMap::new();
    let mut knot_errors = Vec::new();

    for lines in knot_line_sets.into_iter().filter(|lines| !lines.is_empty()) {
        match get_knot_from_lines(lines) {
            Ok((knot_name, knot_data)) => {
                if !knots.contains_key(&knot_name) {
                    knots.insert(knot_name, knot_data);
                } else {
                    let prev_meta_data = knots.get(&knot_name).unwrap().meta_data.clone();

                    knot_errors.push(KnotError {
                        knot_meta_data: knot_data.meta_data.clone(),
                        line_errors: vec![KnotErrorKind::DuplicateKnotName {
                            name: knot_name,
                            prev_meta_data,
                        }],
                    });
                }
            }
            Err(error) => knot_errors.push(error),
        }
    }

    (knots, knot_errors)
}

/// Parse the root knot from a set of lines.
fn parse_root_knot_from_lines(
    lines: Vec<(&str, MetaData)>,
    meta_data: MetaData,
) -> Result<Knot, KnotError> {
    let (_, stitches, line_errors) = get_stitches_from_lines(lines, ROOT_KNOT_NAME);

    if line_errors.is_empty() {
        Ok(Knot {
            default_stitch: ROOT_KNOT_NAME.to_string(),
            stitches,
            tags: Vec::new(),
            meta_data,
        })
    } else {
        Err(KnotError {
            knot_meta_data: meta_data,
            line_errors,
        })
    }
}

/// Parse a single `Knot` from a set of lines.
///
/// Creates `Stitch`es and their node tree of branching content. Returns the knot and its name.
///
/// Assumes that the set of lines is non-empty, which we assert before calling this function.
fn get_knot_from_lines(lines: Vec<(&str, MetaData)>) -> Result<(String, Knot), KnotError> {
    let (head, mut tail) = lines
        .split_first()
        .map(|(head, tail)| (head, tail.to_vec()))
        .unwrap();

    let (head_line, knot_meta_data) = head;

    let mut line_errors = Vec::new();

    let knot_name = match read_knot_name(head_line) {
        Ok(name) => name,
        Err(kind) => {
            let (invalid_name, error) = get_invalid_name_error(head_line, kind, &knot_meta_data);

            line_errors.push(error);

            invalid_name
        }
    };

    if tail.is_empty() {
        line_errors.push(KnotErrorKind::EmptyKnot);
    }

    let tags = get_knot_tags(&mut tail);

    let (default_stitch, stitches, stitch_errors) = get_stitches_from_lines(tail, &knot_name);
    line_errors.extend(stitch_errors);

    if default_stitch.is_some() && line_errors.is_empty() {
        Ok((
            knot_name,
            Knot {
                default_stitch: default_stitch.unwrap(),
                stitches,
                tags,
                meta_data: knot_meta_data.clone(),
            },
        ))
    } else {
        Err(KnotError {
            knot_meta_data: knot_meta_data.clone(),
            line_errors,
        })
    }
}

/// Parse all stitches from a set of lines and return along with encountered errors.
fn get_stitches_from_lines(
    lines: Vec<(&str, MetaData)>,
    knot_name: &str,
) -> (Option<String>, HashMap<String, Stitch>, Vec<KnotErrorKind>) {
    let knot_stitch_sets = divide_lines_at_marker(lines, STITCH_MARKER);

    let mut default_stitch = None;
    let mut stitches = HashMap::new();
    let mut line_errors = Vec::new();

    for (stitch_index, lines) in knot_stitch_sets
        .into_iter()
        .enumerate()
        .filter(|(_, lines)| !lines.is_empty())
    {
        match get_stitch_from_lines(lines, stitch_index, knot_name) {
            Ok((name, stitch)) => {
                if default_stitch.is_none() {
                    default_stitch.replace(name.clone());
                }

                if !stitches.contains_key(&name) {
                    stitches.insert(name, stitch);
                } else {
                    let prev_meta_data = stitches.get(&name).unwrap().meta_data.clone();

                    line_errors.push(KnotErrorKind::DuplicateStitchName {
                        name: name,
                        knot_name: knot_name.to_string(),
                        meta_data: stitch.meta_data.clone(),
                        prev_meta_data,
                    });
                }
            }
            Err(errors) => line_errors.extend(errors),
        }
    }

    (default_stitch, stitches, line_errors)
}

/// Parse a single `Stitch` from a set of lines.
///
/// If a stitch name is found, return it too. This should be found for all stitches except
/// possibly the first in a set, since we split the knot line content where the names are found.
///
/// This function assumes that at least one non-empty line exists in the set, from which
/// the `MetaData` and stitch name (unless it's the root) will be read. This will always be
/// the case, since we split the knot line content at stitch name markers and filter empty
/// lines before calling this.
fn get_stitch_from_lines(
    mut lines: Vec<(&str, MetaData)>,
    stitch_index: usize,
    knot_name: &str,
) -> Result<(String, Stitch), Vec<KnotErrorKind>> {
    let mut line_errors = Vec::new();

    let (first_line, meta_data) = lines[0].clone();

    let stitch_name = match get_stitch_name(first_line, &meta_data) {
        Ok(name) => {
            if name.is_some() {
                lines.remove(0);
            }

            get_stitch_identifier(name, stitch_index)
        }
        Err(kind) => {
            line_errors.push(kind);
            "$INVALID_NAME$".to_string()
        }
    };

    match parse_stitch_from_lines(&lines, knot_name, &stitch_name, meta_data) {
        Ok(stitch) => {
            if line_errors.is_empty() {
                Ok((stitch_name, stitch))
            } else {
                Err(line_errors)
            }
        }
        Err(errors) => {
            line_errors.extend(errors);
            Err(line_errors)
        }
    }
}

/// Read stitch name from the first line in a set.
///
/// If the name was present, return it. If it was not present, return None. If there was
/// another type of error reading the name, return that.
fn get_stitch_name(
    first_line: &str,
    meta_data: &MetaData,
) -> Result<Option<String>, KnotErrorKind> {
    match read_stitch_name(first_line) {
        Ok(name) => Ok(Some(name)),
        Err(KnotNameError::Empty) => Ok(None),
        Err(kind) => Err(KnotErrorKind::InvalidName {
            line: first_line.to_string(),
            kind,
            meta_data: meta_data.clone(),
        }),
    }
}

/// Get an invalid knot name error and a default to use while checking remaining content.
fn get_invalid_name_error(
    line: &str,
    kind: KnotNameError,
    meta_data: &MetaData,
) -> (String, KnotErrorKind) {
    let invalid_name = "$INVALID_NAME$".to_string();

    let error = KnotErrorKind::InvalidName {
        line: line.to_string(),
        kind,
        meta_data: meta_data.clone(),
    };

    (invalid_name, error)
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
fn split_off_prelude_lines<'a>(lines: &mut Vec<(&'a str, MetaData)>) -> Vec<(&'a str, MetaData)> {
    let i = lines
        .iter()
        .position(|(line, _)| line.trim_start().starts_with(KNOT_MARKER))
        .unwrap_or(lines.len());

    lines.drain(..i).collect()
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
fn parse_global_variables(lines: &[(&str, MetaData)]) -> (VariableSet, Vec<PreludeError>) {
    let mut variables = HashMap::new();
    let mut errors = Vec::new();

    for (line, meta_data) in lines
        .iter()
        .map(|(line, meta_data)| (line.trim(), meta_data))
        .filter(|(line, _)| is_variable_line(line))
    {
        if let Err(kind) =
            parse_variable_info_from_line(line, &meta_data).and_then(|(name, variable_info)| {
                match variables.insert(name.clone(), variable_info) {
                    Some(_) => Err(PreludeErrorKind::DuplicateVariable { name }),
                    None => Ok(()),
                }
            })
        {
            errors.push(PreludeError {
                line: line.to_string(),
                kind,
                meta_data: meta_data.clone(),
            });
        }
    }

    (variables, errors)
}

/// Parse a single variable line into the variable name, initial value and whether it is constant.
///
/// Variable lines are on the form `VAR variable_name = initial_value` and constant variables
/// on the form `CONST variable_name = constant_value`.
fn parse_variable_info_from_line(
    line: &str,
    meta_data: &MetaData,
) -> Result<(String, VariableInfo), PreludeErrorKind> {
    if let Some(i) = line.find('=') {
        let (lhs, rhs) = line.split_at(i);

        let is_const = lhs.starts_with(CONST_MARKER);
        let name = parse_variable_name(lhs, is_const)?;
        let variable = parse_variable(rhs.get(1..).unwrap())?;

        Ok((
            name,
            VariableInfo {
                is_const,
                variable,
                meta_data: meta_data.clone(),
            },
        ))
    } else {
        Err(PreludeErrorKind::NoVariableAssignment)
    }
}

/// Check whether or not a line is a variable.
/// 
/// Assumes that the line has been trimmed from both ends.
fn is_variable_line(line: &str) -> bool {
    line.starts_with(VARIABLE_MARKER) || line.starts_with(CONST_MARKER)
}

/// Parse the name from a variable string and assert that it is non-empty.
fn parse_variable_name(lhs: &str, is_const: bool) -> Result<String, PreludeErrorKind> {
    let i = if is_const {
        CONST_MARKER.len()
    } else {
        VARIABLE_MARKER.len()
    };

    lhs.get(i..)
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .ok_or(PreludeErrorKind::NoVariableName)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::{knot::Address, line::Variable};

    pub fn read_knots_from_string(content: &str) -> Result<KnotSet, Vec<KnotError>> {
        let lines = content
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(i, line)| (line, MetaData::from(i)))
            .collect();

        let (knots, knot_errors) = parse_knots_from_lines(lines);

        if knot_errors.is_empty() {
            Ok(knots)
        } else {
            Err(knot_errors)
        }
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
    fn split_lines_into_knots_and_preludes_drains_nothing_if_knots_begin_at_index_zero() {
        let mut lines = enumerate(&["=== knot ==="]);

        assert!(split_off_prelude_lines(&mut lines).is_empty());
        assert_eq!(&denumerate(lines), &["=== knot ==="]);
    }

    #[test]
    fn split_lines_into_knots_and_prelude_drains_all_items_if_knot_is_never_encountered() {
        let mut lines = enumerate(&["No knot here, just prelude content"]);

        split_off_prelude_lines(&mut lines);

        assert!(lines.is_empty());
    }

    #[test]
    fn split_lines_into_knots_and_prelude_drains_lines_up_until_first_knot() {
        let mut lines = enumerate(&[
            "Prelude content ",
            "comes before ",
            "the first named knot.",
            "",
            "=== here ===",
            "Line one.",
        ]);

        let prelude = split_off_prelude_lines(&mut lines);

        assert_eq!(
            &denumerate(prelude),
            &[
                "Prelude content ",
                "comes before ",
                "the first named knot.",
                ""
            ]
        );
        assert_eq!(&denumerate(lines), &["=== here ===", "Line one."]);
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

        let (variables, _) = parse_global_variables(&enumerate(lines));

        assert_eq!(variables.len(), 2);
        assert_eq!(
            variables.get("float").unwrap().variable,
            Variable::Float(1.0)
        );
        assert_eq!(
            variables.get("string").unwrap().variable,
            Variable::String("two words".to_string())
        );
    }

    #[test]
    fn parse_variable_from_line_yields_correct_name() {
        let (name, _) =
            parse_variable_info_from_line("VAR variable = 1.0", &MetaData::from(0)).unwrap();

        assert_eq!(&name, "variable");
    }

    #[test]
    fn parse_variable_from_line_yields_correct_value() {
        let (_, variable_info) =
            parse_variable_info_from_line("VAR variable = 1.0", &MetaData::from(0)).unwrap();

        assert_eq!(variable_info.variable, Variable::from(1.0));
    }

    #[test]
    fn parse_variable_from_line_yields_whether_const_or_not() {
        let (_, non_const_var) =
            parse_variable_info_from_line("VAR variable = 1.0", &MetaData::from(0)).unwrap();
        let (_, const_var) =
            parse_variable_info_from_line("CONST variable = 1.0", &MetaData::from(0)).unwrap();
        
        assert!(!non_const_var.is_const);
        assert!(const_var.is_const);
    }

    #[test]
    fn parse_const_variable_from_line_yields_correct_name_and_value() {
        let (name, const_var) =
            parse_variable_info_from_line("CONST variable = 1.0", &MetaData::from(0)).unwrap();
        
        assert_eq!(&name, "variable");
        assert_eq!(const_var.variable, Variable::from(1.0));
    }

    #[test]
    fn parse_variable_from_line_yields_error_if_no_name() {
        assert!(parse_variable_info_from_line("CONST = 1.0", &MetaData::from(0)).is_err());
    }

    #[test]
    fn parse_variable_from_line_yields_error_if_no_value() {
        assert!(parse_variable_info_from_line("CONST =", &MetaData::from(0)).is_err());
    }

    #[test]
    fn parse_variable_from_line_yields_error_if_no_equal_sign() {
        assert!(parse_variable_info_from_line("CONST variable 1.0", &MetaData::from(0)).is_err());
    }

    #[test]
    fn parse_variable_from_line_yields_error_if_empty_beyond_keyword() {
        assert!(parse_variable_info_from_line("CONST", &MetaData::from(0)).is_err());
        assert!(parse_variable_info_from_line("VAR", &MetaData::from(0)).is_err());
    }

    #[test]
    fn variables_can_be_const_or_not() {
        let lines = &["VAR float = 1.0", "CONST string = \"two words\""];

        let (variables, _) = parse_global_variables(&enumerate(lines));

        let non_const_var = variables.get("float").unwrap();
        let const_var = variables.get("string").unwrap();

        assert!(!non_const_var.is_const);
        assert!(const_var.is_const);
    }

    #[test]
    fn const_variables_are_parsed_identically_to_non_const() {
        let lines = &["VAR non_const_var = 1.0", "CONST const_var = 1.0"];

        let (variables, _) = parse_global_variables(&enumerate(lines));

        let non_const_var = variables.get("non_const_var").unwrap();
        let const_var = variables.get("const_var").unwrap();

        assert_eq!(non_const_var.variable, const_var.variable);
    }

    #[test]
    fn two_variables_with_same_name_yields_error() {
        let lines = &["VAR variable = 1.0", "VAR variable = \"two words\""];

        let (_, errors) = parse_global_variables(&enumerate(lines));

        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn global_variables_are_parsed_with_metadata() {
        let lines = &["VAR float = 1.0", "VAR string = \"two words\""];

        let (variables, _) = parse_global_variables(&enumerate(lines));

        assert_eq!(variables.get("string").unwrap().meta_data, 1.into());
    }

    #[test]
    fn parse_global_variables_returns_all_errors() {
        let lines = &[
            "VAR float = 1.0",
            "VAR = 1.0",                  // no variable name
            "VAR variable = ",            // no assignment
            "VAR variable 10",            // no assignment operator
            "VAR variable = 10chars",     // invalid characters in number
            "VAR variable = \"two words", // unmatched quote marks
            "VAR int = 10",
        ];

        let (variables, errors) = parse_global_variables(&enumerate(lines));

        assert_eq!(variables.len(), 2);
        assert_eq!(errors.len(), 5);
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

        let root = parse_root_knot_from_lines(lines.clone(), ().into()).unwrap();

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

        let root = parse_root_knot_from_lines(lines, ().into()).unwrap();

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

    #[test]
    fn all_prelude_and_knot_errors_are_caught_and_returned() {
        let content = "\
VAR = 0

== knot.stitch
{2 +}
*+  Sticky or non-sticky?

== empty_knot

";

        match read_story_content_from_string(content) {
            Err(ReadError::ParseError(error)) => {
                assert_eq!(error.prelude_errors.len(), 1);
                assert_eq!(error.knot_errors.len(), 2);

                assert_eq!(error.knot_errors[0].line_errors.len(), 3);
                assert_eq!(error.knot_errors[1].line_errors.len(), 1);
            }
            other => panic!("expected `ReadError::ParseError` but got {:?}", other),
        }
    }

    #[test]
    fn reading_story_content_works_if_content_only_has_text() {
        let content = "\
Line one.
";

        assert!(read_story_content_from_string(content).is_ok());
    }

    #[test]
    fn reading_story_content_works_if_content_starts_with_knot() {
        let content = "\
=== knot ===
Line one.
";

        assert!(read_story_content_from_string(content).is_ok());
    }

    #[test]
    fn reading_story_content_does_not_work_if_knot_has_no_content() {
        let content = "\
=== knot ===
";

        assert!(read_story_content_from_string(content).is_err());
    }

    #[test]
    fn reading_story_content_does_not_work_if_stitch_has_no_content() {
        let content = "\
=== knot ===
= stitch
";

        assert!(read_story_content_from_string(content).is_err());
    }

    #[test]
    fn reading_story_content_yields_error_if_duplicate_stitch_names_are_found_in_one_knot() {
        let content = "\
== knot
= stitch
Line one.
= stitch 
Line two.
";

        match read_story_content_from_string(content) {
            Err(ReadError::ParseError(err)) => match &err.knot_errors[0].line_errors[0] {
                KnotErrorKind::DuplicateStitchName { .. } => (),
                other => panic!(
                    "expected `KnotErrorKind::DuplicateStitchName` but got {:?}",
                    other
                ),
            },
            other => panic!("expected `ReadError::ParseError` but got {:?}", other),
        }
    }

    #[test]
    fn reading_story_content_yields_error_if_duplicate_knot_names_are_found() {
        let content = "\
== knot
Line one.
== knot 
Line two.
";

        match read_story_content_from_string(content) {
            Err(ReadError::ParseError(err)) => match &err.knot_errors[0].line_errors[0] {
                KnotErrorKind::DuplicateKnotName { .. } => (),
                other => panic!(
                    "expected `KnotErrorKind::DuplicateKnotName` but got {:?}",
                    other
                ),
            },
            other => panic!("expected `ReadError::ParseError` but got {:?}", other),
        }
    }
}
