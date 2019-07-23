use crate::{
    consts::{KNOT_MARKER, STITCH_MARKER},
    error::{KnotError, KnotNameError},
    follow::{FollowResult, LineDataBuffer, Next},
    line::ParsedLine,
    node::{Follow, RootNode, Stack},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, str::FromStr};

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Knots groups story content into bits. Knots are further subdivided into `Stitch`es,
/// which contain the content.
///
/// Content in `Stitch`es is accessed through the contained hash map which is indexed
/// by their names. Knot content that belongs to the knot itself and not grouped under
/// a named stitch is placed in the map with key `ROOT_KNOT_NAME` (see `consts.rs`).
pub struct Knot {
    /// Name of `Stitch` that is used when diverting to the `Knot` without specifying
    /// a `Stitch`.
    pub default_stitch: String,
    /// Map of `Stitches` belonging to this `Knot`.
    pub stitches: HashMap<String, Stitch>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Stitches contain the actual story content and are grouped in larger `Knot`s.
pub struct Stitch {
    /// Graph of story content, which may or may not branch.
    // pub(crate) root: DialogueNode,
    pub(crate) root: RootNode,
    /// Last recorded position inside the `root` graph of content.
    stack: Stack,
    /// Number of times this stitch has been diverted to.
    pub num_visited: u32,
}

impl Stitch {
    /// Follow a story while reading every line into a buffer.
    pub fn follow(&mut self, buffer: &mut LineDataBuffer) -> FollowResult {
        let result = self.root.follow(&mut self.stack, buffer)?;

        match &result {
            Next::Done | Next::Divert(..) => self.reset_stack(),
            Next::ChoiceSet(..) => (),
        }

        Ok(result)
    }

    /// Follow a story while reading every line into a buffer.
    pub fn follow_with_choice(
        &mut self,
        choice_index: usize,
        buffer: &mut LineDataBuffer,
    ) -> FollowResult {
        let result = self
            .root
            .follow_with_choice(choice_index, 0, &mut self.stack, buffer)?;

        match result {
            Next::Done | Next::Divert(..) => self.reset_stack(),
            _ => (),
        }

        Ok(result)
    }

    pub fn from_lines(lines: &[&str]) -> Result<Self, String> {
        let parsed_lines = lines
            .into_iter()
            .map(|line| ParsedLine::from_str(line).unwrap())
            .collect::<Vec<_>>();

        // let root = DialogueNode::from_lines(&parsed_lines);
        let root = RootNode::from_lines(&parsed_lines);

        Ok(Stitch {
            root,
            stack: vec![0],
            num_visited: 0,
        })
    }

    fn reset_stack(&mut self) {
        self.stack = vec![0];
    }
}

pub fn read_knot_name(line: &str) -> Result<String, KnotError> {
    if line.trim_start().starts_with(KNOT_MARKER) {
        read_name_with_marker(line)
    } else {
        Err(KnotError::InvalidName {
            line: line.to_string(),
            kind: KnotNameError::NoNamePresent,
        }
        .into())
    }
}

pub fn read_stitch_name(line: &str) -> Result<String, KnotError> {
    if line.trim_start().starts_with(STITCH_MARKER) && !line.trim_start().starts_with(KNOT_MARKER) {
        read_name_with_marker(line)
    } else {
        Err(KnotError::InvalidName {
            line: line.to_string(),
            kind: KnotNameError::NoNamePresent,
        }
        .into())
    }
}

fn read_name_with_marker(line: &str) -> Result<String, KnotError> {
    let trimmed_name = line
        .trim_start_matches(STITCH_MARKER)
        .trim_end_matches(STITCH_MARKER)
        .trim();

    if let Some(c) = trimmed_name
        .chars()
        .find(|&c| !(c.is_alphanumeric() || c == '_'))
    {
        let kind = if c.is_whitespace() {
            KnotNameError::ContainsWhitespace
        } else {
            KnotNameError::ContainsInvalidCharacter(c)
        };

        Err(KnotError::InvalidName {
            line: line.to_string(),
            kind,
        }
        .into())
    } else if trimmed_name.is_empty() {
        Err(KnotError::InvalidName {
            kind: KnotNameError::Empty,
            line: line.to_string(),
        })
    } else {
        Ok(trimmed_name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::error::{InklingError, ParseError};

    impl FromStr for Stitch {
        type Err = ParseError;

        fn from_str(content: &str) -> Result<Self, Self::Err> {
            let lines = parse_lines(content)?;
            // let root = DialogueNode::from_lines(&lines);
            let root = RootNode::from_lines(&lines);

            Ok(Stitch {
                root,
                stack: vec![0],
                num_visited: 0,
            })
        }
    }

    fn parse_lines(s: &str) -> Result<Vec<ParsedLine>, ParseError> {
        s.lines().map(|line| ParsedLine::from_str(line)).collect()
    }

    #[test]
    fn knot_restarts_from_their_first_line_when_run_again() {
        let text = "Hello, World!";

        let mut knot = Stitch::from_str(text).unwrap();

        let mut buffer = Vec::new();

        knot.follow(&mut buffer).unwrap();
        knot.follow(&mut buffer).unwrap();

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, text);
        assert_eq!(&buffer[1].text, text);
    }

    #[test]
    fn knot_with_divert_shortcuts_at_it() {
        let name = "fool".to_string();

        let pre = "Mrs. Bennet was making a fool of herself.";
        let after = "After Mrs. Bennet left, Elizabet went upstairs to look after Jane.";

        let text = format!(
            "\
{}
-> {}
{}
",
            pre, name, after
        );

        let mut knot = Stitch::from_str(&text).unwrap();

        let mut buffer = Vec::new();

        assert_eq!(knot.follow(&mut buffer).unwrap(), Next::Divert(name));

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, pre);
        assert_eq!(buffer[1].text.trim(), "");
    }

    #[test]
    fn knot_with_choice_returns_it() {
        let choice1 = "Choice 1";
        let choice2 = "Choice 2";

        let lines = vec![
            "Hello, world!".to_string(),
            format!("* {}", choice1),
            format!("* {}", choice2),
        ];

        let mut text = String::new();
        for line in lines.iter() {
            text.push_str(&line);
            text.push('\n');
        }

        let mut knot = Stitch::from_str(&text).unwrap();

        let mut buffer = Vec::new();

        let choices = match knot.follow(&mut buffer).unwrap() {
            Next::ChoiceSet(choices) => choices,
            _ => panic!("did not get a `ChoiceSet`"),
        };

        assert_eq!(choices.len(), 2);
        assert_eq!(&choices[0].line.text, &choice1);
        assert_eq!(&choices[1].line.text, &choice2);
    }

    #[test]
    fn following_choice_adds_choice_text_to_buffer() {
        let choice = "Choice 1";
        let text = format!("* {}", choice);

        let mut knot = Stitch::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();

        knot.follow(&mut buffer).unwrap();
        knot.follow_with_choice(0, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(&buffer[0].text, choice);
    }

    #[test]
    fn when_a_knot_is_finished_the_stack_is_reset() {
        let text = "\
* Choice 1
* Choice 2
";

        let mut knot = Stitch::from_str(text).unwrap();

        let mut buffer = Vec::new();

        knot.follow(&mut buffer).unwrap();
        assert_eq!(&knot.stack, &[0]);

        knot.follow_with_choice(0, &mut buffer).unwrap();
        assert_eq!(&knot.stack, &[0]);
    }

    #[test]
    fn knot_with_choice_follows_into_choice() {
        let line1 = "A Scandal in Bohemia";
        let line2 = "The Scarlet Letter";
        let line_unused = "Moby Dick; Or, the Whale";

        let lines = vec![
            format!("*  Choice 1"),
            format!("   {}", line_unused),
            format!("*  Choice 2"),
            format!("   {}", line1),
            format!("   {}", line2),
        ];

        let mut text = String::new();
        for line in lines.iter() {
            text.push_str(&line);
            text.push('\n');
        }

        let mut knot = Stitch::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();

        knot.follow(&mut buffer).unwrap();
        knot.follow_with_choice(1, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 3);
        assert_eq!(&buffer[1].text, line1);
        assert_eq!(&buffer[2].text, line2);
    }

    #[test]
    fn knot_gathers_all_choices_at_requested_level() {
        let line1 = "The Thief";
        let line2 = "Sanshirō ";

        let lines = vec![
            format!("*  Choice 1"),
            format!("   The Scarlet Letter"),
            format!("   Moby Dick; Or, the Whale"),
            format!("*  Choice 2"),
            format!("   Den vedervärdige mannen från Säffle"),
            format!("- {}", line1),
            format!("{}", line2),
        ];

        let mut text = String::new();
        for line in lines.iter() {
            text.push_str(&line);
            text.push('\n');
        }

        let mut knot = Stitch::from_str(&text).unwrap();

        let mut results_choice1 = LineDataBuffer::new();

        knot.follow(&mut results_choice1).expect("one");
        knot.follow_with_choice(0, &mut results_choice1)
            .expect("two");

        let mut results_choice2 = LineDataBuffer::new();

        knot.follow(&mut results_choice2).expect("three");
        knot.follow_with_choice(1, &mut results_choice2)
            .expect("four");

        assert_eq!(results_choice1[3], results_choice2[2]);
        assert_eq!(results_choice1[4], results_choice2[3]);
    }

    #[test]
    fn knot_can_follow_multiple_level_choices_and_gathers() {
        let text = "\
Line 1
*   Choice 1
    * *     Choice 1-1
    * *     Choice 1-2
            Line 2
    - -     Line 3
    * *     Choice 1-3
            Line 4
*   Choice 2
-   Line 5
Line 6
";
        let mut knot = Stitch::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();

        knot.follow(&mut buffer).unwrap();
        eprintln!("zero");
        knot.follow_with_choice(0, &mut buffer).unwrap();
        eprintln!("first");
        knot.follow_with_choice(1, &mut buffer).unwrap();
        eprintln!("second");
        knot.follow_with_choice(0, &mut buffer).unwrap();
        eprintln!("third");

        // Four lines in choice, three choice lines and two lines after the gather
        assert_eq!(buffer.len(), 4 + 3 + 2);
    }

    #[test]
    fn follow_returns_error_if_out_of_bounds_index_is_followed_with() {
        let text = "\
*   Choice 1
*   Choice 2
";
        let mut knot = Stitch::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();

        knot.follow(&mut buffer).unwrap();

        match knot.follow_with_choice(2, &mut buffer) {
            Err(InklingError::InvalidChoice {
                index,
                internal_choices,
                ..
            }) => {
                assert_eq!(index, 2);
                assert_eq!(internal_choices.len(), 2);
            }
            _ => panic!("expected a `InklingError::InvalidChoice` but did not get it"),
        }
    }

    #[test]
    fn read_knot_name_from_string_works_with_at_least_two_equal_signs() {
        assert_eq!(&read_knot_name("== Knot").unwrap(), "Knot");
        assert_eq!(&read_knot_name("=== Knot").unwrap(), "Knot");
        assert_eq!(&read_knot_name("== Knot==").unwrap(), "Knot");
        assert_eq!(&read_knot_name("==Knot==").unwrap(), "Knot");
    }

    #[test]
    fn read_stitch_name_from_string_works_with_exactly_one_equal_sign() {
        assert_eq!(&read_stitch_name("= Stitch").unwrap(), "Stitch");
        assert_eq!(&read_stitch_name("=Stitch").unwrap(), "Stitch");
        assert!(&read_stitch_name("== Stitch").is_err());
    }

    #[test]
    fn knot_name_must_be_single_word() {
        assert!(read_knot_name("== Knot name").is_err());
        assert!(read_knot_name("== Knot name ==").is_err());

        match read_knot_name("== knot name") {
            Err(KnotError::InvalidName {
                kind: KnotNameError::ContainsWhitespace,
                ..
            }) => (),
            Err(err) => panic!(
                "Expected a `KnotNameError::ContainsWhitespace` error, got {:?}",
                err
            ),
            _ => panic!("Invalid knot name did not raise error"),
        }
    }

    #[test]
    fn knot_name_cannot_be_empty() {
        assert!(read_knot_name("==").is_err());
        assert!(read_knot_name("== ").is_err());
        assert!(read_knot_name("== a").is_ok());

        match read_knot_name("== ") {
            Err(KnotError::InvalidName {
                kind: KnotNameError::Empty,
                ..
            }) => (),
            err => panic!(
                "expected `KnotNameError::Empty` as kind error, but got {:?}",
                err
            ),
        }
    }

    #[test]
    fn knot_name_can_only_contain_alphanumeric_characters_and_underlines() {
        assert!(read_knot_name("== knot").is_ok());
        assert!(read_knot_name("== knot_name").is_ok());
        assert!(read_knot_name("== knot_name_with_123").is_ok());
        assert!(read_knot_name("== knot_name_with_абв").is_ok());
        assert!(read_knot_name("== knot_name_with_αβγ").is_ok());
        assert!(read_knot_name("== knot_name_with_ñßüåäö").is_ok());
        assert!(read_knot_name("== knot_name_with_京").is_ok());

        assert!(read_knot_name("== knot.name").is_err());
        assert!(read_knot_name("== knot-name").is_err());
        assert!(read_knot_name("== knot/name").is_err());
        assert!(read_knot_name("== knot$name").is_err());

        match read_knot_name("== 京knot.name") {
            Err(KnotError::InvalidName {
                kind: KnotNameError::ContainsInvalidCharacter('.'),
                ..
            }) => (),
            Err(KnotError::InvalidName {
                kind: KnotNameError::ContainsInvalidCharacter(c),
                ..
            }) => panic!(
                "Expected a `KnotNameError::ContainsInvalidCharacter` error \
                 with '.' as contained character, but got '{}'",
                c
            ),
            Err(err) => panic!(
                "Expected a `KnotNameError::ContainsInvalidCharacters` error, got {:?}",
                err
            ),
            _ => panic!("Invalid knot name did not raise error"),
        }
    }

    #[test]
    fn read_knot_name_from_string_returns_error_if_just_one_or_no_equal_signs() {
        assert!(read_knot_name("= Knot name ==").is_err());
        assert!(read_knot_name("=Knot name").is_err());
        assert!(read_knot_name(" Knot name ==").is_err());
        assert!(read_knot_name("Knot name==").is_err());
    }
}
