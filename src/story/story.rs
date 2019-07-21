use crate::{
    consts::{DONE_KNOT, END_KNOT},
    error::{InklingError, ParseError},
    follow::{FollowResult, LineDataBuffer, Next},
    knot::Knot,
    knot::Stitch,
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use super::{
    parse::read_knots_from_string,
    process::{fill_in_invalid_error, prepare_choices_for_user, process_buffer},
};

#[derive(Clone, Debug, PartialEq)]
/// Single line of text in a story, ready to display.
pub struct Line {
    /// Text content.
    pub text: String,
    /// Tags set to the line.
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
/// Choice presented to the user.
pub struct Choice {
    /// Text content.
    pub text: String,
    /// Tags associated with the choice.
    pub tags: Vec<String>,
    /// Internal index of choice in set.
    pub(crate) index: usize,
}

/// Convenience type to indicate when a buffer of `Line` objects is being manipulated.
pub type LineBuffer = Vec<Line>;

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Story with knots, diverts, choices and possibly lots of text.
pub struct Story {
    knots: HashMap<String, Knot>,
    stack: Vec<String>,
    in_progress: bool,
}

#[derive(Debug)]
/// Result from following a `Story`.
///
/// # Examples
/// ```
/// # use inkling::{read_story_from_string, Prompt};
/// let content = "\
/// Professor Lidenbrock had barely a spattering of water left in his flask.
/// *   Axel got the last of it.
/// *   He pressed on, desperately hoping to find water soon.
/// ";
///
/// let mut story = read_story_from_string(content).unwrap();
/// let mut line_buffer = Vec::new();
///
/// match story.start(&mut line_buffer).unwrap() {
///     Prompt::Choice(choice_set) => {
///         println!("Choose:");
///         for (i, choice) in choice_set.iter().enumerate() {
///             println!("{}. {}", i + 1, choice.text);
///         }
///     },
///     Prompt::Done => { /* the story reached its end */ },
/// }
/// ```
pub enum Prompt {
    /// The story reached an end.
    Done,
    /// A choice was encountered.
    Choice(Vec<Choice>),
}

impl Prompt {
    /// If a set of choices was returned, retrieve them without having to match.
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Prompt};
    /// let content = "\
    /// Professor Lidenbrock had barely a spattering of water left in his flask.
    /// *   Axel got the last of it.
    /// *   He pressed on, desperately hoping to find water soon.
    /// ";
    ///
    /// let mut story = read_story_from_string(content).unwrap();
    /// let mut line_buffer = Vec::new();
    ///
    /// if let Some(choices) = story.start(&mut line_buffer).unwrap().get_choices() {
    ///     /* do what you want */
    /// }
    /// ```
    pub fn get_choices(&self) -> Option<Vec<Choice>> {
        match self {
            Prompt::Choice(choices) => Some(choices.clone()),
            _ => None,
        }
    }
}

impl Story {
    /// Start walking through the story while reading all lines into the supplied buffer.
    /// Returns either when the story reached an end or when a set of choices was encountered,
    /// which requires the user to select one. Continue the story with `resume_with_choice`.
    ///
    /// # Notes
    /// The input line buffer is not cleared before reading new lines into it.
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Story};
    /// let content = "\
    /// Only in silence the word,
    /// only in dark the light,
    /// only in dying life:
    /// bright the hawk’s flight
    /// on the empty sky.
    /// ";
    ///
    /// let mut story: Story = read_story_from_string(content).unwrap();
    /// let mut line_buffer = Vec::new();
    ///
    /// story.start(&mut line_buffer);
    ///
    /// assert_eq!(line_buffer.last().unwrap().text, "on the empty sky.\n");
    /// ```
    pub fn start(&mut self, line_buffer: &mut LineBuffer) -> Result<Prompt, InklingError> {
        if self.in_progress {
            return Err(InklingError::StartOnStoryInProgress);
        }

        self.in_progress = true;

        let root_knot_name: String = self
            .stack
            .last()
            .cloned()
            .ok_or::<InklingError>(InklingError::NoKnotStack.into())?;

        self.increment_knot_visit_counter(&root_knot_name)?;

        Self::follow_story_wrapper(
            self,
            |_self, buffer| Self::follow_knot(_self, buffer),
            line_buffer,
        )
    }

    /// Resume the story with a choice from the given set.
    ///
    /// The story continues until it reaches a dead end or another set of choices
    /// is encountered.
    ///
    /// # Notes
    /// The input line buffer is not cleared before reading new lines into it.
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Prompt};
    /// let content = "\
    /// Just as Nancy picked the old diary up from the table she heard
    /// the door behind her creak open. Someone’s coming!
    ///
    /// *   She spun around to face the danger head on.
    ///     Her heart was racing as the door slowly swung open and the black
    ///     cat from the garden swept in.
    ///     “Miao!”   
    /// *   In one smooth motion she hid behind the large curtain.
    ///     A familiar “meow” coming from the room filled her with relief.
    ///     That barely lasted a moment before the dusty curtains made her
    ///     sneeze, awakening the house keeper sleeping in the neighbouring room.
    /// ";
    ///
    /// let mut story = read_story_from_string(content).unwrap();
    /// let mut line_buffer = Vec::new();
    ///
    /// if let Prompt::Choice(choices) = story.start(&mut line_buffer).unwrap() {
    ///     story.resume_with_choice(&choices[0], &mut line_buffer);
    /// }
    ///
    /// assert_eq!(line_buffer.last().unwrap().text, "“Miao!”\n");
    /// ```
    pub fn resume_with_choice(
        &mut self,
        choice: &Choice,
        line_buffer: &mut LineBuffer,
    ) -> Result<Prompt, InklingError> {
        if !self.in_progress {
            return Err(InklingError::ResumeBeforeStart);
        }

        let index = choice.index;

        Self::follow_story_wrapper(
            self,
            |_self, buffer| Self::follow_knot_with_choice(_self, index, buffer),
            line_buffer,
        )
        .map_err(|err| match err {
            InklingError::InvalidChoice { .. } => fill_in_invalid_error(err, &choice, &self.knots),
            _ => err,
        })
    }

    /// Wrapper of common behavior between `start` and `resume_with_choice`. Sets up
    /// a `LineDataBuffer`, reads data into it with the supplied closure and processes
    /// the data by calling `prepare_buffer` on it. If a choice was encountered it
    /// is prepared and returned.
    fn follow_story_wrapper<F>(
        &mut self,
        func: F,
        line_buffer: &mut LineBuffer,
    ) -> Result<Prompt, InklingError>
    where
        F: FnOnce(&mut Self, &mut LineDataBuffer) -> Result<Next, InklingError>,
    {
        let mut internal_buffer = Vec::new();
        let result = func(self, &mut internal_buffer)?;

        process_buffer(line_buffer, internal_buffer);

        match result {
            Next::ChoiceSet(choice_set) => {
                let user_choice_lines = prepare_choices_for_user(&choice_set, &self.knots)?;
                Ok(Prompt::Choice(user_choice_lines))
            }
            Next::Done => Ok(Prompt::Done),
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

    /// Wrapper for common behavior between `follow_knot` and `follow_knot_with_choice`.
    /// Deals with `Diverts` when they are encountered, they are not returned further up
    /// in the call stack.
    fn follow_on_knot_wrapper<F>(&mut self, f: F, buffer: &mut LineDataBuffer) -> FollowResult
    where
        F: FnOnce(&mut Stitch, &mut LineDataBuffer) -> FollowResult,
    {
        let knot_name = self.stack.last().unwrap();

        let result =
            get_mut_stitch(knot_name, &mut self.knots).and_then(|stitch| f(stitch, buffer))?;

        match result {
            Next::Divert(destination) => self.divert_to_knot(&destination, buffer),
            _ => Ok(result),
        }
    }

    /// Update the current stack to a given address and increment the destination's visit counter. 
    /// 
    /// The address may be internal to the current knot, in which case the full address is set. 
    /// For example, if the current knot is called `santiago` and the story wants to divert 
    /// to a stitch with name `cinema` within this knot, the given address `cinema` will set 
    /// the full address as `santiago.cinema` in the stack.
    fn divert_to_knot(&mut self, to_address: &str, buffer: &mut LineDataBuffer) -> FollowResult {
        if to_address == DONE_KNOT || to_address == END_KNOT {
            Ok(Next::Done)
        } else {
            let current_knot = self.stack.last().ok_or(InklingError::NoKnotStack)?;
            let address = get_full_address_of_target(to_address, current_knot, &self.knots)?;

            self.increment_knot_visit_counter(&address)?;

            self.stack.last_mut().map(|knot_name| *knot_name = address);

            self.follow_knot(buffer)
        }
    }

    /// Increment the number of visits counter for the given address. The address must be full.
    fn increment_knot_visit_counter(&mut self, knot_name: &str) -> Result<(), InklingError> {
        get_mut_stitch(knot_name, &mut self.knots)?.num_visited += 1;

        Ok(())
    }
}

/// Read a `Story` by parsing an input string.
///
/// # Examples
/// ```
/// # use inkling::{read_story_from_string, Story};
/// let content = "\
/// He drifted off, and when he opened his eyes the woman was still there.
/// Now she was talking to the old man seated next to her—the farmer from two stations back.
/// ";
///
/// let story: Story = read_story_from_string(content).unwrap();
/// ```
pub fn read_story_from_string(string: &str) -> Result<Story, ParseError> {
    let (root, knots) = read_knots_from_string(string)?;

    Ok(Story {
        knots,
        stack: vec![root],
        in_progress: false,
    })
}

/// Return a reference to the `Stitch` at the target address. 
/// 
/// Addresses are formatted like `[knot].[stitch]`. If only the `[knot]` part is supplied,
/// the default stitch of that `Knot` is returned.
/// 
/// # Note
/// Addresses must be full, not internal within knots. That is, if the story is currently 
/// inside a knot with name `hamburg` and wants to move to a stitch within that knot with 
/// name `date`, the address must be `hamburg.date`, not `date`.
/// 
/// Use `get_full_address_of_target` to get the full address for a stitch within the current
/// knot.
pub fn get_stitch<'a>(
    target: &str,
    knots: &'a HashMap<String, Knot>,
) -> Result<&'a Stitch, InklingError> {
    let (knot_name, stitch_target) = get_divert_address(target);

    knots
        .get(knot_name)
        .and_then(|knot| {
            let stitch_name = stitch_target.unwrap_or(&knot.default_stitch);

            knot.stitches.get(stitch_name)
        })
        .ok_or(InklingError::UnknownDivert {
            knot_name: target.to_string(),
        })
}

/// Return a mutable reference to the `Stitch` at the target address. 
/// 
/// Addresses are formatted like `[knot].[stitch]`. If only the `[knot]` part is supplied,
/// the default stitch of that `Knot` is returned.
/// 
/// # Note
/// Addresses must be full, not internal within knots. That is, if the story is currently 
/// inside a knot with name `hamburg` and wants to move to a stitch within that knot with 
/// name `date`, the address must be `hamburg.date`, not `date`.
/// 
/// Use `get_full_address_of_target` to get the full address for a stitch within the current
/// knot.
pub fn get_mut_stitch<'a>(
    target: &str,
    knots: &'a mut HashMap<String, Knot>,
) -> Result<&'a mut Stitch, InklingError> {
    let (knot_name, stitch_target) = get_divert_address(target);

    knots
        .get_mut(knot_name)
        .and_then(|knot| {
            let stitch_name = stitch_target.unwrap_or(&knot.default_stitch);

            knot.stitches.get_mut(stitch_name)
        })
        .ok_or(InklingError::UnknownDivert {
            knot_name: target.to_string(),
        })
}

/// Split an address on form `[knot].[stitch]` into a tuple. If only the `[knot]` part 
/// is given, return the `[stitch]` part as None.
fn get_divert_address(target: &str) -> (&str, Option<&str>) {
    let items = target.splitn(2, '.').collect::<Vec<_>>();

    if items.len() == 2 {
        (items[0], Some(items[1]))
    } else {
        (items[0], None)
    }
}

/// Get the full address of a stitch. The given address may be internal to the current `Knot`
/// in which case the full address is returned. If the given address is not internal, it is 
/// simply returned.
/// 
/// For example, if we are currently in a knot with name `helsinki` and want to move to 
/// a stitch within it with the name `date_with_kielo`, this function can be given 
/// `date_with_kielo` and return the full address `helsinki.date_with_kielo`. 
/// 
/// If `date_with_kielo` were not a stitch belonging to that knot, just the address 
/// `date_with_kielo` would be returned.
fn get_full_address_of_target(
    target: &str,
    current_address: &str,
    knots: &HashMap<String, Knot>,
) -> Result<String, InklingError> {
    let current_knot = get_knot_part_of_address(current_address);
    let knot = knots.get(current_knot).ok_or(InklingError::NoKnotStack)?;

    if knot.stitches.contains_key(target) {
        Ok(format!("{}.{}", current_knot, target))
    } else {
        Ok(target.to_string())
    }
}

fn get_knot_part_of_address(address: &str) -> &str {
    address
        .find('.')
        .map(|i| address.get(..i).unwrap())
        .unwrap_or(address)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::node::NodeItem;

    #[test]
    fn story_internally_follows_through_knots_when_diverts_are_found() {
        let content = "
== back_in_london
We arrived into London at 9.45pm exactly.
-> hurry_home

== hurry_home 
We hurried home to Savile Row as fast as we could.
";

        let (head_knot, knots) = read_knots_from_string(content).unwrap();

        let mut story = Story {
            knots,
            stack: vec![head_knot],
            in_progress: false,
        };

        let mut buffer = Vec::new();

        story.follow_knot(&mut buffer).unwrap();

        assert_eq!(
            &buffer.last().unwrap().text,
            "We hurried home to Savile Row as fast as we could."
        );
    }

    #[test]
    fn story_internally_resumes_from_the_new_knot_after_a_choice_is_made() {
        let content = "
== back_in_london
We arrived into London at 9.45pm exactly.
-> hurry_home

== hurry_home
\"What's that?\" my master asked.
*	\"I am somewhat tired[.\"],\" I repeated.
\"Really,\" he responded. \"How deleterious.\"
*	\"Nothing, Monsieur!\"[] I replied.
\"Very good, then.\"
*   \"I said, this journey is appalling[.\"] and I want no more of it.\"
\"Ah,\" he replied, not unkindly. \"I see you are feeling frustrated. Tomorrow, things will improve.\"
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let mut story = Story {
            knots,
            stack: vec!["back_in_london".to_string()],
            in_progress: false,
        };

        let mut buffer = Vec::new();

        story.follow_knot(&mut buffer).unwrap();
        story.follow_knot_with_choice(1, &mut buffer).unwrap();

        assert_eq!(&buffer.last().unwrap().text, "\"Very good, then.\"");
    }

    #[test]
    fn when_a_knot_is_returned_to_the_text_starts_from_the_beginning() {
        let content = "
== back_in_london
We arrived into London at 9.45pm exactly.
-> hurry_home

== hurry_home 
*   We hurried home to Savile Row as fast as we could.
*   But we decided our trip wasn't done and immediately left.
After a few days me returned again.
-> back_in_london
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let mut story = Story {
            knots,
            stack: vec!["back_in_london".to_string()],
            in_progress: false,
        };

        let mut buffer = Vec::new();

        story.follow_knot(&mut buffer).unwrap();
        story.follow_knot_with_choice(1, &mut buffer).unwrap();

        assert_eq!(&buffer[0].text, "We arrived into London at 9.45pm exactly.");
        assert_eq!(&buffer[5].text, "We arrived into London at 9.45pm exactly.");
    }

    #[test]
    fn divert_to_done_or_end_constant_knots_ends_story() {
        let content = "
== knot_done
-> DONE 

== knot_end 
-> END
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let mut story = Story {
            knots,
            stack: vec!["knot_done".to_string()],
            in_progress: false,
        };

        let mut buffer = Vec::new();

        match story.start(&mut buffer).unwrap() {
            Prompt::Done => (),
            _ => panic!("story should be done when diverting to DONE knot"),
        }

        story.in_progress = false;
        story.stack = vec!["knot_end".to_string()];

        match story.start(&mut buffer).unwrap() {
            Prompt::Done => (),
            _ => panic!("story should be done when diverting to END knot"),
        }
    }

    #[test]
    fn divert_to_knot_increments_visit_count() {
        let content = "
== knot
Line one.
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let mut buffer = Vec::new();

        let mut story = Story {
            knots,
            stack: vec!["knot".to_string()],
            in_progress: false,
        };

        assert_eq!(get_stitch("knot", &story.knots).unwrap().num_visited, 0);

        story.divert_to_knot("knot", &mut buffer).unwrap();

        assert_eq!(get_stitch("knot", &story.knots).unwrap().num_visited, 1);
    }

    #[test]
    fn divert_to_specific_stitch_sets_stack_to_it() {
        let content = "
== knot
Line one.
= stitch
Line two.
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let mut buffer = Vec::new();

        let mut story = Story {
            knots,
            stack: vec!["knot".to_string()],
            in_progress: false,
        };

        story.divert_to_knot("knot.stitch", &mut buffer).unwrap();
        assert_eq!(story.stack.last().unwrap(), "knot.stitch");
    }

    #[test]
    fn divert_to_stitch_inside_knot_with_internal_target_sets_full_destination_in_stack() {
        let content = "
== knot
Line one.
= stitch
Line two.
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let mut buffer = Vec::new();

        let mut story = Story {
            knots,
            stack: vec!["knot".to_string()],
            in_progress: false,
        };

        story.divert_to_knot("stitch", &mut buffer).unwrap();
        assert_eq!(story.stack.last().unwrap(), "knot.stitch");
    }

    #[test]
    fn starting_a_story_is_only_allowed_once() {
        let mut story = read_story_from_string("Line 1").unwrap();
        let mut line_buffer = Vec::new();

        assert!(story.start(&mut line_buffer).is_ok());

        match story.start(&mut line_buffer) {
            Err(InklingError::StartOnStoryInProgress) => (),
            _ => panic!("did not raise `StartOnStoryInProgress` error"),
        }
    }

    #[test]
    fn cannot_resume_on_a_story_that_has_not_started() {
        let mut story = read_story_from_string("* Choice 1").unwrap();
        let mut line_buffer = Vec::new();

        let choice = Choice {
            index: 0,
            text: "Choice 1".to_string(),
            tags: Vec::new(),
        };

        match story.resume_with_choice(&choice, &mut line_buffer) {
            Err(InklingError::ResumeBeforeStart) => (),
            _ => panic!("did not raise `ResumeBeforeStart` error"),
        }
    }

    #[test]
    fn getting_a_divert_destination_to_knot_returns_default_stitch() {
        let content = "
== knot_one
Knot one

== knot_two
Knot two
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let stitch = get_stitch("knot_one", &knots).unwrap();
        assert_eq!(stitch.root.items.len(), 1);

        match &stitch.root.items[0] {
            NodeItem::Line(line) => assert_eq!(&line.text, "Knot one"),
            _ => panic!(),
        }

        let stitch = get_stitch("knot_two", &knots).unwrap();
        assert_eq!(stitch.root.items.len(), 1);

        match &stitch.root.items[0] {
            NodeItem::Line(line) => assert_eq!(&line.text, "Knot two"),
            _ => panic!(),
        }
    }

    #[test]
    fn divert_destinations_can_be_specific_stitches() {
        let content = "
== knot_one
Knot one
= stitch_one
Stitch one
= stitch_two
Stitch two
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let stitch = get_stitch("knot_one.stitch_two", &knots).unwrap();
        assert_eq!(stitch.root.items.len(), 1);

        match &stitch.root.items[0] {
            NodeItem::Line(line) => assert_eq!(&line.text, "Stitch two"),
            _ => panic!(),
        }
    }

    #[test]
    fn divert_destinations_uses_default_stitch_if_not_specified() {
        let content = "
== knot_one
= stitch_one
Stitch one
= stitch_two
Stitch two
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        let stitch = get_stitch("knot_one", &knots).unwrap();
        assert_eq!(stitch.root.items.len(), 1);

        match &stitch.root.items[0] {
            NodeItem::Line(line) => assert_eq!(&line.text, "Stitch one"),
            _ => panic!(),
        }
    }

    #[test]
    fn divert_to_destinations_raises_error_if_knot_or_stitch_is_not_found() {
        let content = "
== knot_one
= stitch_one
Stitch one
= stitch_two
Stitch two
";

        let (_, knots) = read_knots_from_string(content).unwrap();

        assert!(get_stitch("knot_one", &knots).is_ok());
        assert!(get_stitch("knot_two", &knots).is_err());

        assert!(get_stitch("knot_one.stitch_one", &knots).is_ok());
        assert!(get_stitch("knot_one.stitch_three", &knots).is_err());
    }

    #[test]
    fn get_mutable_access_to_stitches_works_similarly() {
        let content = "
== knot_one
Knot one

== knot_two
= stitch_one
Knot two
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();

        {
            let stitch = get_mut_stitch("knot_one", &mut knots).unwrap();
            stitch.num_visited += 1;
        }

        assert_eq!(get_stitch("knot_one", &knots).unwrap().num_visited, 1);

        assert!(get_mut_stitch("knot_one", &mut knots).is_ok());
        assert!(get_mut_stitch("knot_two", &mut knots).is_ok());
        assert!(get_mut_stitch("knot_two.stitch_one", &mut knots).is_ok());

        assert!(get_mut_stitch("knot_three", &mut knots).is_err());
        assert!(get_mut_stitch("knot_one.stitch_five", &mut knots).is_err());
    }
}
