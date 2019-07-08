use std::{collections::HashMap, str::FromStr};

use crate::{
    error::FollowError,
    line::{Choice, Line, LineKind, ParsedLine},
    node::{DialogueNode, Stack},
};

#[derive(Debug)]
pub struct Knot {
    root: DialogueNode,
    stack: Stack,
    prev_choice_set: Vec<Choice>,
}

pub type LineBuffer = Vec<Line>;

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum Next {
    /// Finished with the current node or story.
    Done,
    /// Divert to a new knot with the given name.
    Divert(String),
    /// Choice for the user.
    ChoiceSet(Vec<Choice>),
}

impl Knot {
    /// Follow a story while reading every line into a buffer.
    fn follow(&mut self, buffer: &mut LineBuffer) -> Result<Next, FollowError> {
        let result = self.root.follow(0, buffer, &mut self.stack);

        match &result {
            Ok(Next::ChoiceSet(choices)) => self.prev_choice_set = choices.clone(),
            _ => (),
        }

        result
    }

    /// Follow a story while reading every line into a buffer.
    fn follow_with_choice(
        &mut self,
        choice_index: usize,
        buffer: &mut LineBuffer,
    ) -> Result<Next, FollowError> {
        let choice = self
            .prev_choice_set
            .get(choice_index)
            .ok_or(FollowError::InvalidChoice {
                selection: choice_index,
                num_choices: self.prev_choice_set.len(),
            })?;

        buffer.push(choice.line.clone());

        self.root
            .follow_with_choice(choice_index, 0, buffer, &mut self.stack)
    }

    /// Follow a story while reading every line into a pure text buffer,
    /// discarding other data.
    fn follow_into_string(&mut self, buffer: &mut String) -> Result<Next, FollowError> {
        let mut line_buffer = Vec::new();
        let result = self.follow(&mut line_buffer)?;

        for line in line_buffer {
            buffer.push_str(&line.text);

            if !line.glue_end {
                buffer.push('\n');
            }
        }

        Ok(result)
    }
}

impl FromStr for Knot {
    type Err = String;

    fn from_str(content: &str) -> Result<Self, Self::Err> {
        let lines = parse_lines(content)?;
        let root = DialogueNode::from_lines(&lines);

        Ok(Knot {
            root,
            stack: Vec::new(),
            prev_choice_set: Vec::new(),
        })
    }
}

fn parse_lines(s: &str) -> Result<Vec<ParsedLine>, String> {
    s.lines().map(|line| ParsedLine::from_str(line)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knot_from_plain_text_lines_fully_replicates_them() {
        let text = "\
Hello, world!
Hello?
Hello, are you there?
";

        let mut knot = Knot::from_str(text).unwrap();

        let mut buffer = String::new();

        assert_eq!(knot.follow_into_string(&mut buffer).unwrap(), Next::Done);
        assert_eq!(buffer, text);
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

        let mut knot = Knot::from_str(&text).unwrap();
        eprintln!("{:#?}", &knot);

        let mut buffer = String::new();

        assert_eq!(
            knot.follow_into_string(&mut buffer).unwrap(),
            Next::Divert(name)
        );
        assert_eq!(buffer.trim_end(), pre);
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

        let mut knot = Knot::from_str(&text).unwrap();

        let mut buffer = String::new();

        let choices = match knot.follow_into_string(&mut buffer).unwrap() {
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

        let mut knot = Knot::from_str(&text).unwrap();

        let mut buffer = LineBuffer::new();

        knot.follow(&mut buffer).unwrap();
        knot.follow_with_choice(0, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(&buffer[0].text, choice);
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

        let mut knot = Knot::from_str(&text).unwrap();

        let mut buffer = LineBuffer::new();

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

        let mut knot = Knot::from_str(&text).unwrap();

        let mut results_choice1 = LineBuffer::new();

        knot.follow(&mut results_choice1).unwrap();
        knot.follow_with_choice(0, &mut results_choice1).unwrap();
        knot.stack.clear();

        let mut results_choice2 = LineBuffer::new();

        knot.follow(&mut results_choice2).unwrap();
        knot.follow_with_choice(1, &mut results_choice2).unwrap();
        knot.stack.clear();

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
        let mut knot = Knot::from_str(&text).unwrap();

        let mut buffer = LineBuffer::new();

        knot.follow(&mut buffer).unwrap();
        knot.follow_with_choice(0, &mut buffer).unwrap();
        knot.follow_with_choice(1, &mut buffer).unwrap();
        knot.follow_with_choice(0, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 6 + 3);
    }

    #[test]
    fn follow_returns_error_if_out_of_bounds_index_is_followed_with() {
        let text = "\
*   Choice 1
*   Choice 2
";
        let mut knot = Knot::from_str(&text).unwrap();

        let mut buffer = LineBuffer::new();

        knot.follow(&mut buffer).unwrap();

        match knot.follow_with_choice(2, &mut buffer) {
            Err(FollowError::InvalidChoice { selection: 2, num_choices: 2 }) => (),
            Err(FollowError::InvalidChoice { .. }) => panic!(),
            _ => panic!("expected a `FollowError::InvalidChoice` but did not get it"),
        }
    }
}
