use crate::{
    consts::{CHOICE_MARKER, DIVERT_MARKER, GLUE_MARKER, TAG_MARKER},
};

use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum LineKind {
    /// Move on with the story.
    Regular,
    /// Divert to a new knot with the given name.
    Divert(String),
    // Choice for the user.
    // Choice(Choice),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub selection_text: String,
    pub line: Line,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub kind: LineKind,
    pub tags: Vec<String>,
    pub glue_start: bool,
    pub glue_end: bool,
}

enum ParsedLine {
    Choice(Choice),
    Line(Line),
}

impl FromStr for Line {
    type Err = ();

    fn from_str(line: &str) -> Result<Self, Self::Err> {


        let mut text = line.to_string();

        let tags = parse_tags(&mut text);
        let divert = parse_divert(&mut text);

        text = text.trim().to_string();

        // Diverts always act as glue
        let (glue_start, glue_end) = parse_line_glue(&mut text, divert.is_some());

        let kind = if let Some(name) = divert {
            LineKind::Divert(name)
        } else {
            LineKind::Regular
        };

        Ok(Line {
            text,
            kind,
            tags,
            glue_start,
            glue_end,
        })
    }
}

// impl Line {
//     pub fn from_string(line: &str) -> Line {
//         let mut text = line.to_string();

//         let tags = parse_tags(&mut text);
//         let divert = parse_divert(&mut text);

//         text = text.trim().to_string();

//         // Diverts always act as glue
//         let (glue_start, glue_end) = parse_line_glue(&mut text, divert.is_some());

//         let kind = if let Some(name) = divert {
//             LineKind::Divert(name)
//         } else {
//             LineKind::Regular
//         };

//         Line {
//             text,
//             kind,
//             tags,
//             glue_start,
//             glue_end,
//         }
//     }
// }

/// Parse and remove glue markers from either side, retaining enclosed whitespace. 
/// A divert always acts as right glue.
fn parse_line_glue(line: &mut String, has_divert: bool) -> (bool, bool) {
    let glue_left = line.starts_with(GLUE_MARKER);
    let glue_right = line.ends_with(GLUE_MARKER) || has_divert;

    if glue_left {
        *line = line.trim_start_matches(GLUE_MARKER).to_string();
    }

    if glue_right {
        *line = line.trim_end_matches(GLUE_MARKER).to_string();
    }

    (glue_left, glue_right)
}

/// Split diverts off the given line and return it separately if found.
fn parse_divert(line: &mut String) -> Option<String> {
    match line.find(DIVERT_MARKER) {
        Some(i) => {
            let part = line.split_off(i);

            part.trim_start_matches(DIVERT_MARKER)
                .split(DIVERT_MARKER)
                .map(|knot_name| knot_name.trim().to_string())
                .next()
        }
        None => None,
    }
}

/// Split any found tags off the given line and return them separately.
fn parse_tags(line: &mut String) -> Vec<String> {
    match line.find(TAG_MARKER) {
        Some(i) => {
            let part = line.split_off(i);

            part.trim_matches(TAG_MARKER)
                .split(TAG_MARKER)
                .map(|tag| tag.to_string())
                .collect::<Vec<_>>()
        }
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_simple_line() {
        let text = "Hello, world!";

        let line = Line::from_str(text).unwrap();

        assert_eq!(&line.text, text);
        assert_eq!(line.kind, LineKind::Regular);
    }

    #[test]
    fn read_line_trims_whitespace() {
        let text = "   Hello, world!   ";
        let line = Line::from_str(text).unwrap();

        assert_eq!(&line.text, text.trim());
    }

    #[test]
    fn line_with_glue_retains_whitespace_on_side() {
        let text = "Hello, world!";
        let whitespace = "    ";

        let line_with_left_glue = format!(
            "{marker}{pad}{text}",
            pad = &whitespace,
            text = &text,
            marker = GLUE_MARKER
        );

        let line_with_right_glue = format!(
            "{text}{pad}{marker}",
            text = &text,
            pad = &whitespace,
            marker = GLUE_MARKER
        );

        let line_left = Line::from_str(&line_with_left_glue).unwrap();

        assert_eq!(line_left.text, format!("{}{}", &whitespace, &text));
        assert!(line_left.glue_start);
        assert!(!line_left.glue_end);

        let line_right = Line::from_str(&line_with_right_glue).unwrap();

        assert_eq!(line_right.text, format!("{}{}", &text, &whitespace));
        assert!(!line_right.glue_start);
        assert!(line_right.glue_end);
    }

    #[test]
    fn divert_line_returns_knot_name() {
        let name = "knot_name";
        let text = format!("-> {}", name);

        let line = Line::from_str(&text).unwrap();
        
        assert_eq!(&line.text, "");
        assert_eq!(line.kind, LineKind::Divert(name.to_string()));
    }

    #[test]
    fn embedded_divert_returns_knot_name() {
        let head = "Hello, world!";
        let name = "knot_name";
        let text = format!("{}->{}", head, name);

        let line = Line::from_str(&text).unwrap();
        assert_eq!(&line.text, head);
        assert_eq!(line.kind, LineKind::Divert(name.to_string()));
    }

    #[test]
    fn diverts_in_lines_acts_as_glue() {
        let head = "Hello, world! ";
        let name = "knot_name";
        let text = format!("{}->{}", head, name);

        let line = Line::from_str(&text).unwrap();

        assert!(line.glue_end);
    }

    #[test]
    fn tags_are_not_added_if_none_are_given() {
        let head = "Hello, world! ";
        let name = "knot_name";
        let text = format!("{}->{}", head, name);

        let line = Line::from_str(&text).unwrap();
        assert!(line.tags.is_empty());
    }

    #[test]
    fn multiple_tags_can_be_specified() {
        let head = "Hello, world!";

        let tag1 = "blue colour".to_string();
        let tag2 = "transparent".to_string();
        let tag3 = "italic text".to_string();

        let text = format!(
            "{head}{sep}{}{sep}{}{sep}{}",
            tag1,
            tag2,
            tag3,
            head = head,
            sep = TAG_MARKER
        );

        let line = Line::from_str(&text).unwrap();
        let tags = &line.tags;

        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0], tag1);
        assert_eq!(tags[1], tag2);
        assert_eq!(tags[2], tag3);
    }

    #[test]
    fn parse_choice_with_text() {
        let head = "Choice text";
        let text = format!(" {}{}", CHOICE_MARKER, head);

        let line = Line::from_str(&text).unwrap();

        assert!(line.text.is_empty());

        match line.kind {
            // LineKind::Choice(choice) => {
            //     assert_eq!(&choice.choice_text.text, head);
            //     assert_eq!(&choice.display_text.text, head);
            // },
            _ => panic!("line is not a choice"),
        }
    }
}
