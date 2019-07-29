//! Structures which contain parsed `Ink` stories and content presented to the user.

use crate::{
    consts::{DONE_KNOT, END_KNOT},
    error::{InklingError, ParseError, StackError},
    follow::{ChoiceInfo, EncounteredEvent, LineDataBuffer},
    knot::{Knot, Stitch},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use super::{
    address::Address,
    parse::read_knots_from_string,
    process::{get_fallback_choices, prepare_choices_for_user, process_buffer},
};

#[derive(Clone, Debug, PartialEq)]
/// Single line of text in a story, ready to display.
pub struct Line {
    /// Text to display.
    ///
    /// The text is ready to be printed as-is, without the addition of more characters.
    /// It been processed to remove extraneous whitespaces and contains a newline character
    /// at the end of the line unless the line was glued to the next.
    pub text: String,
    /// Tags set to the line.
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Choice presented to the user.
pub struct Choice {
    /// Line of text to represent the choice with.
    ///
    /// The text is ready to be printed as-is. It contains no newline character at the end.
    pub text: String,
    /// Tags associated with the choice.
    pub tags: Vec<String>,
    /// Internal index of choice in set.
    pub(crate) index: usize,
}

/// Convenience type to indicate when a buffer of `Line` objects is being manipulated.
pub type LineBuffer = Vec<Line>;

/// Convenience type for a set of `Knot`s.
///
/// The knot names are used as keys in the collection.
pub type Knots = HashMap<String, Knot>;

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Story with knots, diverts, choices and possibly lots of text.
pub struct Story {
    /// Collection of `Knot`s which make up the story.
    knots: Knots,
    /// Internal stack for which `Knot` is actively being followed.
    stack: Vec<Address>,
    /// Set of last choices presented to the user.
    last_choices: Option<Vec<Choice>>,
    /// Whether or not the story has been started.
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
    ///
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

        let initial_address = self.get_current_address()?;
        get_mut_stitch(&initial_address, &mut self.knots)?.num_visited += 1;

        self.follow_story_wrapper(None, line_buffer)
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
    ///     story.resume_with_choice(0, &mut line_buffer);
    /// }
    ///
    /// assert_eq!(line_buffer.last().unwrap().text, "“Miao!”\n");
    /// ```
    pub fn resume_with_choice(
        &mut self,
        selection: usize,
        line_buffer: &mut LineBuffer,
    ) -> Result<Prompt, InklingError> {
        if !self.in_progress {
            return Err(InklingError::ResumeBeforeStart);
        }

        let index = self
            .last_choices
            .as_ref()
            .ok_or(StackError::NoLastChoices.into())
            .and_then(|last_choices| {
                last_choices
                    .get(selection)
                    .ok_or(InklingError::InvalidChoice {
                        selection,
                        presented_choices: last_choices.clone(),
                    })
                    .map(|choice| choice.index)
            })?;

        self.follow_story_wrapper(Some(index), line_buffer)
    }

    /// Wrapper of common behavior between `start` and `resume_with_choice`.
    ///
    /// Updates the stack to the last visited address and the last presented set of choices
    /// if encountered.
    fn follow_story_wrapper(
        &mut self,
        selection: Option<usize>,
        line_buffer: &mut LineBuffer,
    ) -> Result<Prompt, InklingError> {
        let current_address = self.get_current_address()?;

        let (result, last_address) =
            follow_story(&current_address, line_buffer, selection, &mut self.knots)?;

        self.update_last_stack(&last_address);

        match result {
            Prompt::Choice(choices) => {
                self.last_choices.replace(choices.clone());

                Ok(Prompt::Choice(choices))
            }
            other => Ok(other),
        }
    }

    /// Get the current address from the stack.
    fn get_current_address(&self) -> Result<Address, InklingError> {
        self.stack.last().cloned().ok_or(StackError::NoStack.into())
    }

    /// Set the given address as active on the stack.
    fn update_last_stack(&mut self, address: &Address) {
        self.stack.push(address.clone());
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

    let root_address = Address::from_root_knot(&root, &knots).expect(
        "After successfully creating all knots, the root knot name that was returned from \
         `read_knots_from_string` is not present in the set of created knots. \
         This should never happen.",
    );

    Ok(Story {
        knots,
        stack: vec![root_address],
        last_choices: None,
        in_progress: false,
    })
}

/// Follow the nodes in a story with selected branch index if supplied.
///
/// When an event that triggers a `Prompt` is encountered it will be returned along with
/// the last visited address. Lines that are followed in the story will be processed
/// and added to the input buffer.
fn follow_story(
    current_address: &Address,
    line_buffer: &mut LineBuffer,
    selection: Option<usize>,
    knots: &mut Knots,
) -> Result<(Prompt, Address), InklingError> {
    let (internal_buffer, last_address, event) = follow_knot(current_address, selection, knots)?;

    process_buffer(line_buffer, internal_buffer);

    match event {
        EncounteredEvent::BranchingChoice(choice_set) => {
            let user_choice_lines = prepare_choices_for_user(&choice_set, &current_address, knots)?;
            if !user_choice_lines.is_empty() {
                Ok((Prompt::Choice(user_choice_lines), last_address))
            } else {
                let choice = get_fallback_choice(&choice_set, &current_address, knots)?;

                follow_story(current_address, line_buffer, Some(choice.index), knots)
            }
        }
        EncounteredEvent::Done => Ok((Prompt::Done, last_address)),
        EncounteredEvent::Divert(..) => unreachable!("diverts are treated in `follow_knot`"),
    }
}

/// Follow a knot through diverts.
///
/// Will [follow][crate::node::Follow] through the story starting from the input address
/// and return all encountered lines. Diverts will be automatically moved to.
///
/// The function returns when either a branching point is encountered or there is no
/// content left to follow. When it returns it will return with the last visited address.
fn follow_knot(
    address: &Address,
    mut selection: Option<usize>,
    knots: &mut Knots,
) -> Result<(LineDataBuffer, Address, EncounteredEvent), InklingError> {
    let mut buffer = Vec::new();
    let mut current_address = address.clone();

    let event = loop {
        let current_stitch = get_mut_stitch(&current_address, knots)?;

        let result = match selection.take() {
            Some(i) => current_stitch.follow_with_choice(i, &mut buffer),
            None => current_stitch.follow(&mut buffer),
        }?;

        match result {
            EncounteredEvent::Divert(ref to_address)
                if to_address == END_KNOT || to_address == DONE_KNOT =>
            {
                break EncounteredEvent::Done
            }
            EncounteredEvent::Divert(ref to_address) => {
                current_address =
                    Address::from_target_address(to_address, &current_address, knots)?;

                let knot = get_mut_stitch(&current_address, knots)?;
                knot.num_visited += 1;
            }
            _ => break result,
        }
    };

    Ok((buffer, current_address, event))
}

/// Return a reference to the `Stitch` at the target address.
pub fn get_stitch<'a>(target: &Address, knots: &'a Knots) -> Result<&'a Stitch, InklingError> {
    knots
        .get(&target.knot)
        .and_then(|knot| knot.stitches.get(&target.stitch))
        .ok_or(
            StackError::BadAddress {
                address: target.clone(),
            }
            .into(),
        )
}

/// Return a mutable reference to the `Stitch` at the target address.
pub fn get_mut_stitch<'a>(
    target: &Address,
    knots: &'a mut Knots,
) -> Result<&'a mut Stitch, InklingError> {
    knots
        .get_mut(&target.knot)
        .and_then(|knot| knot.stitches.get_mut(&target.stitch))
        .ok_or(
            StackError::BadAddress {
                address: target.clone(),
            }
            .into(),
        )
}

/// Return the first available fallback choice from the given set of choices.
///
/// Choices are filtered as usual by conditions and visits.
fn get_fallback_choice(
    choice_set: &[ChoiceInfo],
    current_address: &Address,
    knots: &Knots,
) -> Result<Choice, InklingError> {
    get_fallback_choices(choice_set, current_address, knots).and_then(|choices| {
        choices.first().cloned().ok_or(InklingError::OutOfChoices {
            address: current_address.clone(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follow_knot_diverts_to_new_knots_when_encountered() {
        let content = "
== back_in_london
We arrived into London at 9.45pm exactly.
-> hurry_home

== hurry_home
We hurried home to Savile Row as fast as we could.
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let (buffer, _, _) = follow_knot(&root_address, None, &mut knots).unwrap();

        assert_eq!(
            &buffer.last().unwrap().text,
            "We hurried home to Savile Row as fast as we could."
        );
    }

    #[test]
    fn follow_knot_does_not_return_divert_event_if_divert_is_encountered() {
        let content = "
== back_in_london
We arrived into London at 9.45pm exactly.
-> hurry_home

== hurry_home
We hurried home to Savile Row as fast as we could.
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let (_, _, event) = follow_knot(&root_address, None, &mut knots).unwrap();

        match event {
            EncounteredEvent::Done => (),
            other => panic!("expected `EncounteredEvent::Done` but got {:?}", other),
        }
    }

    #[test]
    fn follow_knot_returns_choices_when_encountered() {
        let content = "
== select_destination
*   Tripoli
*   Addis Ababa
*   Rabat
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        let root_address = Address::from_root_knot("select_destination", &knots).unwrap();

        let (_, _, event) = follow_knot(&root_address, None, &mut knots).unwrap();

        match event {
            EncounteredEvent::BranchingChoice(ref choices) => {
                assert_eq!(choices.len(), 3);
            }
            other => panic!(
                "expected `EncounteredEvent::BranchingChoice` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn follow_knot_returns_the_last_active_knot() {
        let content = "
== back_in_london
We arrived into London at 9.45pm exactly.
-> hurry_home

== hurry_home
We hurried home to Savile Row as fast as we could.
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let (_, last_address, _) = follow_knot(&root_address, None, &mut knots).unwrap();

        assert_eq!(
            last_address,
            Address::from_root_knot("hurry_home", &knots).unwrap()
        );
    }

    #[test]
    fn divert_to_done_or_end_constant_knots_ends_story() {
        let content = "
== knot_done
-> DONE

== knot_end
-> END
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        let done_address = Address::from_root_knot("knot_done", &knots).unwrap();
        let end_address = Address::from_root_knot("knot_end", &knots).unwrap();

        match follow_knot(&done_address, None, &mut knots).unwrap() {
            (_, _, EncounteredEvent::Done) => (),
            _ => panic!("story should be done when diverting to DONE knot"),
        }

        match follow_knot(&end_address, None, &mut knots).unwrap() {
            (_, _, EncounteredEvent::Done) => (),
            _ => panic!("story should be done when diverting to END knot"),
        }
    }

    #[test]
    fn divert_to_knot_increments_its_visit_count() {
        let content = "
== addis_ababa
-> tripoli

== tripoli
-> DONE
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();

        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();
        let divert_address = Address::from_root_knot("tripoli", &knots).unwrap();

        assert_eq!(get_stitch(&divert_address, &knots).unwrap().num_visited, 0);

        follow_knot(&current_address, None, &mut knots).unwrap();

        assert_eq!(get_stitch(&divert_address, &knots).unwrap().num_visited, 1);
    }

    #[test]
    fn knots_do_not_get_their_number_of_visits_incremented_when_resuming_a_choice() {
        let content = "
== tripoli

*   Cinema -> END
*   Visit family -> END
";
        let (_, mut knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_root_knot("tripoli", &knots).unwrap();

        assert_eq!(get_stitch(&current_address, &knots).unwrap().num_visited, 0);

        follow_knot(&current_address, Some(1), &mut knots).unwrap();

        assert_eq!(get_stitch(&current_address, &knots).unwrap().num_visited, 0);
    }

    #[test]
    fn follow_story_returns_last_visited_address_when_reaching_end() {
        let content = "
== addis_ababa
-> tripoli.cinema

== tripoli

= cinema
-> END

= visit_family
-> END
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();

        let mut line_buffer = Vec::new();
        let (_, last_address) =
            follow_story(&current_address, &mut line_buffer, None, &mut knots).unwrap();

        assert_eq!(
            last_address,
            Address::from_target_address("tripoli.cinema", &current_address, &knots).unwrap()
        );
    }

    #[test]
    fn follow_story_returns_last_visited_address_when_encountering_choices() {
        let content = "
== addis_ababa
-> tripoli

== tripoli

*   Cinema -> END
*   Visit family -> END
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();

        let mut line_buffer = Vec::new();
        let (_, last_address) =
            follow_story(&current_address, &mut line_buffer, None, &mut knots).unwrap();

        assert_eq!(
            last_address,
            Address::from_target_address("tripoli", &current_address, &knots).unwrap()
        );
    }

    #[test]
    fn following_story_wrapper_updates_stack_to_last_address() {
        let content = "
== addis_ababa
-> tripoli.cinema

== tripoli

= cinema
-> END

= visit_family
-> END
";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.follow_story_wrapper(None, &mut line_buffer).unwrap();

        let address = Address::from_target_address(
            "tripoli.cinema",
            &story.get_current_address().unwrap(),
            &story.knots,
        )
        .unwrap();

        assert_eq!(story.stack.last().unwrap(), &address);
    }

    #[test]
    fn choice_index_is_used_to_resume_story_with() {
        let content = "
== tripoli
*   Cinema
    You watched a horror movie in the cinema.
*   Visit family
    A visit to your family did you good.
";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();
        story.resume_with_choice(1, &mut line_buffer).unwrap();

        assert_eq!(
            &line_buffer[1].text,
            "A visit to your family did you good.\n"
        );
    }

    #[test]
    fn choice_index_is_converted_to_internal_branch_index_to_account_for_filtered_choices() {
        let content = "
== tripoli
*   Cinema
    You watched a horror movie in the cinema.
*   {addis_ababa} Call Kinfe.
*   Visit family
    A visit to your family did you good.

== addis_ababa
-> END
";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();
        story.resume_with_choice(1, &mut line_buffer).unwrap();

        assert_eq!(
            &line_buffer[1].text,
            "A visit to your family did you good.\n"
        );
    }

    #[test]
    fn if_choice_list_returned_to_user_is_empty_follow_fallback_choice() {
        let content = "
== knot
*   Non-sticky choice -> knot
*   ->
    Fallback choice
";

        let mut story = read_story_from_string(content).unwrap();

        let mut buffer = Vec::new();

        let choices = story.start(&mut buffer).unwrap().get_choices().unwrap();
        assert_eq!(choices.len(), 1);

        story.resume_with_choice(0, &mut buffer).unwrap();

        assert_eq!(&buffer[1].text, "Fallback choice\n");
    }

    #[test]
    fn if_no_fallback_choices_are_available_raise_error() {
        let content = "
== knot
*   Non-sticky choice -> knot
";

        let mut story = read_story_from_string(content).unwrap();

        let mut buffer = Vec::new();

        story.start(&mut buffer).unwrap();

        match story.resume_with_choice(0, &mut buffer) {
            Err(InklingError::OutOfChoices { .. }) => (),
            Err(err) => panic!("expected `OutOfChoices` error but got {:?}", err),
            Ok(_) => panic!("expected an error but got an Ok"),
        }
    }

    #[test]
    fn last_set_of_presented_choices_are_stored() {
        let content = "
== knot 
*   Choice 1
*   Choice 2
";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        assert!(story.last_choices.is_none());

        story.start(&mut line_buffer).unwrap();

        let last_choices = story.last_choices.as_ref().unwrap();

        assert_eq!(last_choices.len(), 2);
        assert_eq!(&last_choices[0].text, "Choice 1");
        assert_eq!(&last_choices[1].text, "Choice 2");
    }

    #[test]
    fn when_an_invalid_choices_is_made_to_resume_the_story_an_invalid_choice_error_is_yielded() {
        let content = "
== knot 
*   Choice 1
";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();

        match story.resume_with_choice(1, &mut line_buffer) {
            Err(InklingError::InvalidChoice {
                selection,
                presented_choices,
            }) => {
                assert_eq!(selection, 1);
                assert_eq!(presented_choices.len(), 1);
                assert_eq!(&presented_choices[0].text, "Choice 1");
            }
            other => panic!("expected `InklingError::InvalidChoice` but got {:?}", other),
        }
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

        match story.resume_with_choice(0, &mut line_buffer) {
            Err(InklingError::ResumeBeforeStart) => (),
            _ => panic!("did not raise `ResumeBeforeStart` error"),
        }
    }

    #[test]
    fn when_a_knot_is_returned_to_the_text_starts_from_the_beginning() {
        let content = "
== back_in_almaty
We arrived into Almaty at 9.45pm exactly.
-> hurry_home

== hurry_home
*   We hurried home as fast as we could. -> END
*   But we decided our trip wasn't done and immediately left.
    After a few days me returned again.
    -> back_in_almaty
";

        let mut story = read_story_from_string(content).unwrap();

        let mut line_buffer = Vec::new();

        story.follow_story_wrapper(None, &mut line_buffer).unwrap();
        story
            .follow_story_wrapper(Some(1), &mut line_buffer)
            .unwrap();

        assert_eq!(
            &line_buffer[0].text,
            "We arrived into Almaty at 9.45pm exactly.\n"
        );
        assert_eq!(
            &line_buffer[3].text,
            "We arrived into Almaty at 9.45pm exactly.\n"
        );
    }

    #[test]
    fn when_the_story_begins_the_first_knot_gets_its_number_of_visits_set_to_one() {
        let content = "
Hello, World!
";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();

        let address = Address::from_root_knot("$ROOT$", &story.knots).unwrap();

        assert_eq!(get_stitch(&address, &story.knots).unwrap().num_visited, 1);
    }
}
