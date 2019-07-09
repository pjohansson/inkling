use crate::{
    error::{FollowError, InternalError},
    follow::{FollowResult, LineDataBuffer, Next},
    knot::Knot,
};

use std::{collections::HashMap, str::FromStr};

use super::process::{prepare_choices_for_user, process_buffer};

#[derive(Debug)]
/// Single line of text in a story, ready to display.
pub struct Line {
    /// Text content.
    pub text: String,
    /// Tags set to the line.
    pub tags: Vec<String>,
}

/// Convenience type to indicate when a buffer of `Line` objects is being manipulated.
pub type LineBuffer = Vec<Line>;

#[derive(Debug)]
/// Story with knots, diverts, choices and possibly lots of text.
pub struct Story {
    knots: HashMap<String, Knot>,
    stack: Vec<String>,
}

/// Result from following a `Story`.
pub enum StoryAction {
    /// The story reached an end.
    Done,
    /// A choice was encountered.
    Choice(Vec<Line>),
}

impl Story {
    /// Start walking through the story while reading all lines into the supplied buffer.
    /// Returns either when the story reached an end or when a set of choices was encountered,
    /// which requires the user to select one. Continue the story with `resume_with_choice`.
    pub fn start(&mut self, line_buffer: &mut LineBuffer) -> Result<StoryAction, FollowError> {
        Self::follow_story_wrapper(
            self,
            |_self, buffer| Self::follow_knot(_self, buffer),
            line_buffer,
        )
    }

    /// Resume the story with the choice corresponding to the input `index`. Indexing starts
    /// from 0, so the third choice in a set will have index 2.
    ///
    /// The story continues until it reaches a dead end or another set of choices
    /// is encountered.
    pub fn resume_with_choice(
        &mut self,
        index: usize,
        line_buffer: &mut LineBuffer,
    ) -> Result<StoryAction, FollowError> {
        Self::follow_story_wrapper(
            self,
            |_self, buffer| Self::follow_knot_with_choice(_self, index, buffer),
            line_buffer,
        )
    }

    fn follow_story_wrapper<F>(
        &mut self,
        func: F,
        line_buffer: &mut LineBuffer,
    ) -> Result<StoryAction, FollowError>
    where
        F: FnOnce(&mut Self, &mut LineDataBuffer) -> Result<Next, FollowError>,
    {
        let mut internal_buffer = Vec::new();
        let result = func(self, &mut internal_buffer)?;

        process_buffer(line_buffer, internal_buffer);

        match result {
            Next::ChoiceSet(choice_set) => {
                let user_choice_lines = prepare_choices_for_user(&choice_set);
                Ok(StoryAction::Choice(user_choice_lines))
            }
            Next::Done => Ok(StoryAction::Done),
            Next::Divert(..) => unreachable!("diverts are treated in the closure"),
        }
    }

    /* Internal functions to walk through the story into a `LineDataBuffer`
     * which will be processed into the user supplied lines by the public functions */

    fn follow_knot(&mut self, line_buffer: &mut LineDataBuffer) -> FollowResult {
        self.follow_on_knot_wrapper(|knot, buffer| knot.follow(buffer), line_buffer)
    }

    fn follow_knot_with_choice(
        &mut self,
        choice_index: usize,
        line_buffer: &mut LineDataBuffer,
    ) -> FollowResult {
        self.follow_on_knot_wrapper(
            |knot, buffer| knot.follow_with_choice(choice_index, buffer),
            line_buffer,
        )
    }

    fn follow_on_knot_wrapper<F>(&mut self, f: F, buffer: &mut LineDataBuffer) -> FollowResult
    where
        F: FnOnce(&mut Knot, &mut LineDataBuffer) -> FollowResult,
    {
        let knot_name = self.stack.last().unwrap();

        let result = self
            .knots
            .get_mut(knot_name)
            .ok_or(
                InternalError::UnknownKnot {
                    name: knot_name.clone(),
                }
                .into(),
            )
            .and_then(|knot| f(knot, buffer));

        match result {
            Ok(Next::Divert(to_knot)) => {
                self.stack.last_mut().map(|knot_name| *knot_name = to_knot);

                self.follow_knot(buffer)
            }
            _ => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn following_story_removes_empty_lines_through_choices_and_diverts() {
        let knot1_name = "back_in_london";
        let knot2_name = "hurry_outside";
        let knot3_name = "dragged_outside";
        let knot4_name = "as_fast_as_we_could";

        let knot1_lines = vec![
            "We arrived into London at 9.45pm exactly.",
            "",
            "*	\"There is not a moment to lose!\" I declared.",
            "	-> hurry_outside ",
            "",
            "*	\"Monsieur, let us savour this moment!\" I declared.",
            "	My master clouted me firmly around the head and dragged me out of the door. ",
            "	-> dragged_outside ",
            "",
            "*	[We hurried home] -> hurry_outside",
            "",
        ];

        let knot2_lines = vec!["We hurried home to Savile Row -> as_fast_as_we_could", ""];

        let knot3_lines = vec![
            "He insisted that we hurried home to Savile Row ",
            "-> as_fast_as_we_could",
        ];

        let knot4_lines = vec!["<> as fast as we could."];

        let mut knots = HashMap::new();

        knots.insert(
            knot1_name.to_string(),
            Knot::from_str(&knot1_lines.join("\n")).unwrap(),
        );
        knots.insert(
            knot2_name.to_string(),
            Knot::from_str(&knot2_lines.join("\n")).unwrap(),
        );
        knots.insert(
            knot3_name.to_string(),
            Knot::from_str(&knot3_lines.join("\n")).unwrap(),
        );
        knots.insert(
            knot4_name.to_string(),
            Knot::from_str(&knot4_lines.join("\n")).unwrap(),
        );

        let mut story = Story {
            knots,
            stack: vec![knot1_name.to_string()],
        };

        let mut buffer = Vec::new();

        story.start(&mut buffer).unwrap();
        story.resume_with_choice(1, &mut buffer).unwrap();

        let buffer_as_string =
            buffer
                .iter()
                .map(|line| line.text.clone())
                .fold(String::new(), |mut acc, line| {
                    acc.push_str(&line);
                    acc
                });

        let buffer_lines = buffer_as_string.lines().collect::<Vec<_>>();
        assert_eq!(buffer_lines.len(), 4);
        assert_eq!(
            buffer_lines[3],
            "He insisted that we hurried home to Savile Row as fast as we could."
        );
    }

    #[test]
    fn story_internally_follows_through_knots_when_diverts_are_found() {
        let knot1_name = "back_in_london".to_string();
        let knot2_name = "hurry_home".to_string();

        let knot1_text = format!(
            "\
We arrived into London at 9.45pm exactly.
-> {}\
",
            knot2_name
        );

        let knot2_text = format!(
            "\
             We hurried home to Savile Row as fast as we could.\
             "
        );

        let mut knots = HashMap::new();

        knots.insert(knot1_name.clone(), Knot::from_str(&knot1_text).unwrap());
        knots.insert(knot2_name, Knot::from_str(&knot2_text).unwrap());

        let mut story = Story {
            knots,
            stack: vec![knot1_name],
        };

        let mut buffer = Vec::new();

        story.follow_knot(&mut buffer).unwrap();

        assert_eq!(&buffer.last().unwrap().text, &knot2_text);
    }

    #[test]
    fn story_internally_resumes_from_the_new_knot_after_a_choice_is_made() {
        let knot1_name = "back_in_london".to_string();
        let knot2_name = "hurry_home".to_string();

        let knot1_text = format!(
            "\
We arrived into London at 9.45pm exactly.
-> {}\
",
            knot2_name
        );

        let knot2_text = format!(
            "\
\"What's that?\" my master asked.
*	\"I am somewhat tired[.\"],\" I repeated.
	\"Really,\" he responded. \"How deleterious.\"
*	\"Nothing, Monsieur!\"[] I replied.
	\"Very good, then.\"
*   \"I said, this journey is appalling[.\"] and I want no more of it.\"
	\"Ah,\" he replied, not unkindly. \"I see you are feeling frustrated. Tomorrow, things will improve.\"\
"
        );

        let mut knots = HashMap::new();

        knots.insert(knot1_name.clone(), Knot::from_str(&knot1_text).unwrap());
        knots.insert(knot2_name, Knot::from_str(&knot2_text).unwrap());

        let mut story = Story {
            knots,
            stack: vec![knot1_name],
        };

        let mut buffer = Vec::new();

        story.follow_knot(&mut buffer).unwrap();
        story.follow_knot_with_choice(1, &mut buffer).unwrap();

        assert_eq!(&buffer.last().unwrap().text, "\"Very good, then.\"");
    }

    #[test]
    fn when_a_knot_is_returned_to_the_text_starts_from_the_beginning() {
        let knot1_name = "back_in_london".to_string();
        let knot2_name = "hurry_home".to_string();

        let knot1_line = "We arrived into London at 9.45pm exactly.";

        let knot1_text = format!(
            "\
{}
-> {}\
",
            knot1_line, knot2_name
        );

        let knot2_text = format!(
            "\
*   We hurried home to Savile Row as fast as we could. 
*   But we decided our trip wasn't done and immediately left.
    After a few days me returned again.
    -> {}\
",
            knot1_name
        );

        let mut knots = HashMap::new();

        knots.insert(knot1_name.clone(), Knot::from_str(&knot1_text).unwrap());
        knots.insert(knot2_name, Knot::from_str(&knot2_text).unwrap());

        let mut story = Story {
            knots,
            stack: vec![knot1_name],
        };

        let mut buffer = Vec::new();

        story.follow_knot(&mut buffer).unwrap();
        story.follow_knot_with_choice(1, &mut buffer).unwrap();

        assert_eq!(&buffer[0].text, knot1_line);
        assert_eq!(&buffer[5].text, knot1_line);
    }
}
