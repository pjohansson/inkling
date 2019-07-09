use crate::{
    error::FollowError,
    follow::{FollowResult, LineDataBuffer, Next},
    line::{Choice, ParsedLine},
    node::{DialogueNode, Stack},
};

use std::str::FromStr;

#[derive(Debug)]
pub struct Knot {
    pub(crate) root: DialogueNode,
    stack: Stack,
    prev_choice_set: Vec<Choice>,
}

impl Knot {
    /// Follow a story while reading every line into a buffer.
    pub fn follow(&mut self, buffer: &mut LineDataBuffer) -> FollowResult {
        let result = self.root.follow(0, buffer, &mut self.stack)?;

        match &result {
            Next::ChoiceSet(choices) => self.prev_choice_set = choices.clone(),
            Next::Done | Next::Divert(..) => self.stack.clear(),
        }

        Ok(result)
    }

    /// Follow a story while reading every line into a buffer.
    pub fn follow_with_choice(
        &mut self,
        choice_index: usize,
        buffer: &mut LineDataBuffer,
    ) -> FollowResult {
        self.add_choice_to_buffer(choice_index, buffer)?;

        let result = self
            .root
            .follow_with_choice(choice_index, 0, buffer, &mut self.stack)?;

        match result {
            Next::Done | Next::Divert(..) => self.stack.clear(),
            _ => (),
        }

        Ok(result)
    }

    pub fn from_lines(lines: &[&str]) -> Result<Self, String> {
        let parsed_lines = lines
            .into_iter()
            .map(|line| ParsedLine::from_str(line).unwrap())
            .collect::<Vec<_>>();
        let root = DialogueNode::from_lines(&parsed_lines);

        Ok(Knot {
            root,
            stack: Vec::new(),
            prev_choice_set: Vec::new(),
        })
    }

    fn add_choice_to_buffer(
        &self,
        choice_index: usize,
        buffer: &mut LineDataBuffer,
    ) -> Result<(), FollowError> {
        let choice = self
            .prev_choice_set
            .get(choice_index)
            .ok_or(FollowError::InvalidChoice {
                selection: choice_index,
                num_choices: self.prev_choice_set.len(),
            })?;

        buffer.push(choice.line.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn knot_restarts_from_their_first_line_when_run_again() {
        let text = "Hello, World!";

        let mut knot = Knot::from_str(text).unwrap();

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

        let mut knot = Knot::from_str(&text).unwrap();

        let mut buffer = Vec::new();

        assert_eq!(knot.follow(&mut buffer).unwrap(), Next::Divert(name));

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, pre);
        assert_eq!(&buffer[1].text, "");
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

        let mut knot = Knot::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();

        knot.follow(&mut buffer).unwrap();
        knot.follow_with_choice(0, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(&buffer[0].text, choice);
    }

    #[test]
    fn when_a_knot_is_finished_after_a_choice_the_stack_is_reset() {
        let text = "\
* Choice 1
* Choice 2
";

        let mut knot = Knot::from_str(text).unwrap();

        let mut buffer = Vec::new();

        knot.follow(&mut buffer).unwrap();
        assert!(!knot.stack.is_empty());

        knot.follow_with_choice(0, &mut buffer).unwrap();
        assert!(knot.stack.is_empty());
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

        let mut knot = Knot::from_str(&text).unwrap();

        let mut results_choice1 = LineDataBuffer::new();

        knot.follow(&mut results_choice1).unwrap();
        knot.follow_with_choice(0, &mut results_choice1).unwrap();
        knot.stack.clear();

        let mut results_choice2 = LineDataBuffer::new();

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

        let mut buffer = LineDataBuffer::new();

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

        let mut buffer = LineDataBuffer::new();

        knot.follow(&mut buffer).unwrap();

        match knot.follow_with_choice(2, &mut buffer) {
            Err(FollowError::InvalidChoice {
                selection: 2,
                num_choices: 2,
            }) => (),
            Err(FollowError::InvalidChoice { .. }) => panic!(),
            _ => panic!("expected a `FollowError::InvalidChoice` but did not get it"),
        }
    }
}
