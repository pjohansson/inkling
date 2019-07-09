use crate::{
    consts::{
        KNOT_MARKER, LINE_COMMENT_MARKER, ROOT_KNOT_NAME, STITCH_MARKER, TODO_COMMENT_MARKER,
    },
    error::{KnotError, ParseError},
    knot::Knot,
};

use std::collections::HashMap;

pub fn read_knots_from_string(
    content: &str,
) -> Result<(String, HashMap<String, Knot>), ParseError> {
    let all_lines = content.lines().collect::<Vec<_>>();
    let content_lines = remove_empty_and_comment_lines(all_lines);
    let knot_line_sets = divide_into_knots(content_lines);

    if knot_line_sets.is_empty() {
        return Err(ParseError::Empty);
    }

    let results = knot_line_sets
        .into_iter()
        .enumerate()
        .map(|(i, lines)| {
            if i == 0 {
                get_first_knot_from_lines(&lines)
            } else {
                get_knot_from_lines(&lines)
            }
        })
        .collect::<Result<Vec<_>, ParseError>>()?;

    let root = results[0].0.clone();

    let mut knots = HashMap::new();

    for (name, knot) in results {
        knots.insert(name, knot);
    }

    Ok((root, knots))
}

/// First node in story is allowed to not have a name, treat that separately.
fn get_first_knot_from_lines(lines: &[&str]) -> Result<(String, Knot), ParseError> {
    if lines.len() < 1 {
        return Err(KnotError::Empty.into());
    }

    let name = read_first_knot_name(&lines[0]);

    let i = if name == ROOT_KNOT_NAME { 0 } else { 1 };

    let knot = Knot::from_lines(&lines[i..]).unwrap();

    Ok((name, knot))
}

fn get_knot_from_lines(lines: &[&str]) -> Result<(String, Knot), ParseError> {
    if lines.len() <= 1 {
        return Err(KnotError::Empty.into());
    }

    let name = read_knot_name(&lines[0])?;
    let knot = Knot::from_lines(&lines[1..]).unwrap();

    Ok((name, knot))
}

fn divide_into_knots(mut content: Vec<&str>) -> Vec<Vec<&str>> {
    let mut buffer = Vec::new();

    while let Some(i) = content
        .iter()
        .rposition(|line| line.trim_start().starts_with(KNOT_MARKER))
    {
        buffer.push(content.split_off(i));
    }

    buffer.push(content);
    buffer.into_iter().rev().collect()
}

fn read_knot_name(line: &str) -> Result<String, ParseError> {
    if line.trim_start().starts_with(KNOT_MARKER) {
        Ok(line
            .trim_start_matches(STITCH_MARKER)
            .trim_end_matches(STITCH_MARKER)
            .trim()
            .to_string())
    } else {
        Err(KnotError::NoName {
            string: line.to_string(),
        }
        .into())
    }
}

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
        .collect()
}

fn read_first_knot_name(line: &str) -> String {
    read_knot_name(line).unwrap_or(ROOT_KNOT_NAME.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn divide_into_knots_splits_given_string_at_knot_markers() {
        let content = vec![
            "",
            "== Knot one ",
            "Line 1",
            "Line 2",
            "",
            "=== Knot two ===",
            "Line 3",
            "",
        ];

        let knot_lines = divide_into_knots(content.clone());

        assert_eq!(knot_lines[0][..], content[0..1]);
        assert_eq!(knot_lines[1][..], content[1..5]);
        assert_eq!(knot_lines[2][..], content[5..]);
    }

    #[test]
    fn read_knot_name_from_string_works_with_at_least_two_equal_signs() {
        assert_eq!(&read_knot_name("== Knot name").unwrap(), "Knot name");
        assert_eq!(&read_knot_name("=== Knot name").unwrap(), "Knot name");
        assert_eq!(&read_knot_name("== Knot name ==").unwrap(), "Knot name");
        assert_eq!(&read_knot_name("==Knot name==").unwrap(), "Knot name");
    }

    #[test]
    fn read_knot_name_from_string_returns_error_if_just_one_or_no_equal_signs() {
        assert!(read_knot_name("= Knot name ==").is_err());
        assert!(read_knot_name("=Knot name").is_err());
        assert!(read_knot_name(" Knot name ==").is_err());
        assert!(read_knot_name("Knot name==").is_err());
    }

    #[test]
    fn empty_lines_and_comment_lines_are_removed_by_initial_processing() {
        let content = vec![
            "Good line",
            "// Comment line is remove",
            "TODO: As is todo comments",
            "TODO but not without a semi colon!",
        ];

        let lines = remove_empty_and_comment_lines(content.clone());
        assert_eq!(&lines, &[content[0].clone(), content[3].clone()]);
    }

    #[test]
    fn first_set_of_knot_lines_gets_default_name_if_not_given() {
        assert_eq!(&read_first_knot_name("== Knot name"), "Knot name");
        assert_eq!(&read_first_knot_name("Knot name"), ROOT_KNOT_NAME);
    }

    #[test]
    fn parsing_knot_from_line_list_gets_name_and_knot() {
        let content = vec!["== Knot name ==", "Line 1", "Line 2"];

        let (name, knot) = get_knot_from_lines(&content).unwrap();
        assert_eq!(&name, "Knot name");
        assert_eq!(knot.root.items.len(), 2);
    }

    #[test]
    fn parsing_first_knot_from_line_list_takes_first_line_as_content_if_no_knot_no_is_present() {
        let content = vec!["== Knot name", "Line 1", "Line 2"];

        let (name, knot) = get_first_knot_from_lines(&content).unwrap();
        assert_eq!(&name, "Knot name");
        assert_eq!(knot.root.items.len(), 2);

        let content = vec!["Line 1", "Line 2"];

        let (name, knot) = get_first_knot_from_lines(&content).unwrap();
        assert_eq!(&name, ROOT_KNOT_NAME);
        assert_eq!(knot.root.items.len(), 2);
    }
}
