//! Structures which contain parsed `Ink` stories and content presented to the user.

use crate::{
    error::{InklingError, ParseError, StackError},
    follow::{ChoiceInfo, EncounteredEvent, FollowData, LineDataBuffer},
    knot::{get_empty_knot_counts, get_mut_stitch, validate_addresses_in_knots, Address, KnotSet},
    process::{get_fallback_choices, prepare_choices_for_user, process_buffer},
    story::read_knots_from_string,
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

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
    /// The text is ready to be printed as-is. It is trimmed of whitespace from both ends
    /// and contains no newline character at the end.
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
    /// Collection of `Knot`s which make up the story.
    knots: KnotSet,
    /// Internal stack for which `Knot` is actively being followed.
    stack: Vec<Address>,
    /// Internal data for the story.
    data: FollowData,
    /// Set of last choices presented to the user.
    last_choices: Option<Vec<Choice>>,
    /// Whether a choice has to be made.
    requires_choice: bool,
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
    /// // From ‘A Wizard of Earthsea’ by Ursula K. Le Guin
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

        if !self.requires_choice {
            return Err(InklingError::ResumeWithoutChoice);
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

        self.requires_choice = false;

        self.follow_story_wrapper(Some(index), line_buffer)
    }

    /// Move the story to another knot or stitch.
    ///
    /// A move can be performed at any time, before or after starting the story. It
    /// simply updates the current internal address in the story to the given address.
    /// If no stitch name is given the default stitch from the root will be selected.
    ///
    /// The story will not resume automatically after the move. To do this, use
    /// the `resume` method whenever you are ready.
    ///
    /// # Examples
    /// ```
    /// // From ‘Purge’ by Sofi Oksanen
    /// # use inkling::{read_story_from_string, Prompt};
    /// let content = "\
    /// May, 1949
    /// For the free Estonia!
    ///
    /// === chapter_one ===
    /// 1992, western Estonia
    /// Aliide Truu stared at the fly and the fly stared right back at her.
    /// ";
    ///
    /// let mut story = read_story_from_string(content).unwrap();
    /// let mut line_buffer = Vec::new();
    ///
    /// // Let’s skip ahead!
    /// story.move_to("chapter_one", None).unwrap();
    /// story.start(&mut line_buffer).unwrap();
    ///
    /// assert_eq!(&line_buffer[0].text, "1992, western Estonia\n");
    /// ```
    pub fn move_to(&mut self, knot: &str, stitch: Option<&str>) -> Result<(), InklingError> {
        let to_address = Address::from_parts(knot, stitch, &self.knots).map_err(|_| {
            InklingError::InvalidAddress {
                knot: knot.to_string(),
                stitch: stitch.map(|s| s.to_string()),
            }
        })?;

        self.update_last_stack(&to_address);

        self.requires_choice = false;
        self.last_choices = None;

        Ok(())
    }

    /// Resume the story after a move.
    ///
    /// Starts walking through the story from the knot and stitch that the story was moved
    /// to. Works exactly like `start`, except that this cannot be called before the story
    /// has been started (mirroring how `start` cannot be called on a story in progress).
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Prompt};
    /// # let content = "\
    /// # Sam was in real trouble now. The fleet footed criminals were just about to corner her.
    /// #
    /// # *   [Climb the fire escape]
    /// #     She clattered up the rackety fire escape.
    /// # *   [Bluff]
    /// #     To hell with running. Sam turned around with a cocksure smirk on her lips.
    /// #  
    /// # === mirandas_den ===
    /// # = bar
    /// # The room was thick with smoke and the smell of noir.
    /// #
    /// # = meeting
    /// # Miranda was waiting in her office.
    /// # She had questions and Sam for once had answers.
    /// # ";
    /// # let mut story = read_story_from_string(content).unwrap();
    /// # let mut line_buffer = Vec::new();
    /// # story.start(&mut line_buffer).unwrap();
    /// story.move_to("mirandas_den", Some("meeting")).unwrap();
    /// # line_buffer.clear();
    /// story.resume(&mut line_buffer).unwrap();
    ///
    /// assert_eq!(&line_buffer[0].text, "Miranda was waiting in her office.\n");
    /// ```
    pub fn resume(&mut self, line_buffer: &mut LineBuffer) -> Result<Prompt, InklingError> {
        if !self.in_progress {
            return Err(InklingError::ResumeBeforeStart);
        }

        self.follow_story_wrapper(None, line_buffer)
    }

    /// Get the tags associated with the given knot.
    ///
    /// Returns an error if no knot with the given name exists in the story.
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Prompt};
    /// // From ‘Sanshirō’ by Natsume Sōseki
    /// let content = "
    /// === tokyo ===
    /// ## weather: hot
    /// ## sound: crowds
    /// Tokyo was full of things that startled Sanshirō.
    /// ";
    ///
    /// let story = read_story_from_string(content).unwrap();
    /// let tags = story.get_knot_tags("tokyo").unwrap();
    ///
    /// assert_eq!(&tags[0], "weather: hot");
    /// assert_eq!(&tags[1], "sound: crowds");
    /// ```
    pub fn get_knot_tags(&self, knot_name: &str) -> Result<Vec<String>, InklingError> {
        self.knots
            .get(knot_name)
            .map(|knot| knot.tags.clone())
            .ok_or(InklingError::InvalidAddress {
                knot: knot_name.to_string(),
                stitch: None,
            })
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

        let mut internal_buffer = Vec::new();

        let (result, last_address) = follow_story(
            &current_address,
            &mut internal_buffer,
            selection,
            &mut self.knots,
            &mut self.data,
        )?;

        process_buffer(line_buffer, internal_buffer);

        self.update_last_stack(&last_address);

        match result {
            Prompt::Choice(choices) => {
                self.last_choices.replace(choices.clone());
                self.requires_choice = true;

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
    let (root_knot_name, mut knots) = read_knots_from_string(string)?;

    validate_addresses_in_knots(&mut knots)?;

    let root_address = Address::from_root_knot(&root_knot_name, &knots).expect(
        "After successfully creating all knots, the root knot name that was returned from \
         `read_knots_from_string` is not present in the set of created knots. \
         This simply should not be possible",
    );

    let data = FollowData {
        knot_visit_counts: get_empty_knot_counts(&knots),
    };

    Ok(Story {
        knots,
        stack: vec![root_address],
        data,
        last_choices: None,
        requires_choice: false,
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
    internal_buffer: &mut LineDataBuffer,
    selection: Option<usize>,
    knots: &mut KnotSet,
    data: &mut FollowData,
) -> Result<(Prompt, Address), InklingError> {
    let (last_address, event) =
        follow_knot(current_address, internal_buffer, selection, knots, data)?;

    match event {
        EncounteredEvent::BranchingChoice(choice_set) => {
            let user_choice_lines = prepare_choices_for_user(&choice_set, data)?;
            if !user_choice_lines.is_empty() {
                Ok((Prompt::Choice(user_choice_lines), last_address))
            } else {
                let choice = get_fallback_choice(&choice_set, &last_address, data)?;

                follow_story(
                    &last_address,
                    internal_buffer,
                    Some(choice.index),
                    knots,
                    data,
                )
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
    internal_buffer: &mut LineDataBuffer,
    mut selection: Option<usize>,
    knots: &mut KnotSet,
    data: &mut FollowData,
) -> Result<(Address, EncounteredEvent), InklingError> {
    let mut current_address = address.clone();

    let event = loop {
        let current_stitch = get_mut_stitch(&current_address, knots)?;

        let result = match selection.take() {
            Some(i) => current_stitch.follow_with_choice(i, internal_buffer, data),
            None => current_stitch.follow(internal_buffer, data),
        }?;

        match result {
            EncounteredEvent::Divert(Address::End) => break EncounteredEvent::Done,
            EncounteredEvent::Divert(to_address) => {
                current_address = to_address;
            }
            _ => break result,
        }
    };

    Ok((current_address, event))
}

/// Return the first available fallback choice from the given set of choices.
///
/// Choices are filtered as usual by conditions and visits.
fn get_fallback_choice(
    choice_set: &[ChoiceInfo],
    current_address: &Address,
    data: &FollowData,
) -> Result<Choice, InklingError> {
    get_fallback_choices(choice_set, data).and_then(|choices| {
        choices.first().cloned().ok_or(InklingError::OutOfChoices {
            address: current_address.clone(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        consts::ROOT_KNOT_NAME,
        knot::{get_num_visited, ValidateAddresses},
    };

    fn mock_follow_data(knots: &KnotSet) -> FollowData {
        FollowData {
            knot_visit_counts: get_empty_knot_counts(knots),
        }
    }

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
        validate_addresses_in_knots(&mut knots).unwrap();
        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        follow_knot(&root_address, &mut buffer, None, &mut knots, &mut data).unwrap();

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
        validate_addresses_in_knots(&mut knots).unwrap();
        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        let (_, event) =
            follow_knot(&root_address, &mut buffer, None, &mut knots, &mut data).unwrap();

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
        validate_addresses_in_knots(&mut knots).unwrap();
        let root_address = Address::from_root_knot("select_destination", &knots).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        let (_, event) =
            follow_knot(&root_address, &mut buffer, None, &mut knots, &mut data).unwrap();

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
        validate_addresses_in_knots(&mut knots).unwrap();
        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        let (last_address, _) =
            follow_knot(&root_address, &mut buffer, None, &mut knots, &mut data).unwrap();

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
        validate_addresses_in_knots(&mut knots).unwrap();
        let done_address = Address::from_root_knot("knot_done", &knots).unwrap();
        let end_address = Address::from_root_knot("knot_end", &knots).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        match follow_knot(&done_address, &mut buffer, None, &mut knots, &mut data).unwrap() {
            (_, EncounteredEvent::Done) => (),
            _ => panic!("story should be done when diverting to DONE knot"),
        }

        match follow_knot(&end_address, &mut buffer, None, &mut knots, &mut data).unwrap() {
            (_, EncounteredEvent::Done) => (),
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
        validate_addresses_in_knots(&mut knots).unwrap();

        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();
        let divert_address = Address::from_root_knot("tripoli", &knots).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        follow_knot(&current_address, &mut buffer, None, &mut knots, &mut data).unwrap();

        assert_eq!(get_num_visited(&divert_address, &data).unwrap(), 1);
    }

    #[test]
    fn knots_do_not_get_their_number_of_visits_incremented_when_resuming_a_choice() {
        let content = "
== tripoli

*   Cinema -> END
*   Visit family -> END
";
        let (_, mut knots) = read_knots_from_string(content).unwrap();
        validate_addresses_in_knots(&mut knots).unwrap();
        let current_address = Address::from_root_knot("tripoli", &knots).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        follow_knot(
            &current_address,
            &mut buffer,
            Some(1),
            &mut knots,
            &mut data,
        )
        .unwrap();

        assert_eq!(get_num_visited(&current_address, &data).unwrap(), 0);
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
        validate_addresses_in_knots(&mut knots).unwrap();
        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();

        let mut line_buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        let (_, last_address) = follow_story(
            &current_address,
            &mut line_buffer,
            None,
            &mut knots,
            &mut data,
        )
        .unwrap();

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
        validate_addresses_in_knots(&mut knots).unwrap();
        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();

        let mut line_buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        let (_, last_address) = follow_story(
            &current_address,
            &mut line_buffer,
            None,
            &mut knots,
            &mut data,
        )
        .unwrap();

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
    fn fallback_choices_resume_from_the_knot_they_are_encountered_in() {
        let content = "
== first
-> second 

== second 
+   -> 
    Fallback choice
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        validate_addresses_in_knots(&mut knots).unwrap();
        let current_address = Address::from_root_knot("first", &knots).unwrap();

        let mut line_buffer = Vec::new();
        let mut internal_buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        follow_story(
            &current_address,
            &mut internal_buffer,
            None,
            &mut knots,
            &mut data,
        )
        .unwrap();
        process_buffer(&mut line_buffer, internal_buffer);

        assert_eq!(&line_buffer[0].text, "Fallback choice\n");
    }

    #[test]
    fn glue_is_followed_over_fallback_choices() {
        let content = "
== tripoli
We decided to go to the <> 
*   [] Cinema.
";

        let (_, mut knots) = read_knots_from_string(content).unwrap();
        validate_addresses_in_knots(&mut knots).unwrap();
        let current_address = Address::from_root_knot("tripoli", &knots).unwrap();

        let mut line_buffer = Vec::new();
        let mut internal_buffer = Vec::new();
        let mut data = mock_follow_data(&knots);

        follow_story(
            &current_address,
            &mut internal_buffer,
            None,
            &mut knots,
            &mut data,
        )
        .unwrap();
        process_buffer(&mut line_buffer, internal_buffer);

        assert_eq!(&line_buffer[0].text, "We decided to go to the ");
        assert_eq!(&line_buffer[1].text, "Cinema.\n");
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

        assert_eq!(get_num_visited(&address, &story.data).unwrap(), 1);
    }

    #[test]
    fn all_addresses_are_validated_in_knots_after_reading_story_from_lines() {
        let content = "

== back_in_almaty
We arrived into Almaty at 9.45pm exactly.
-> hurry_home

== hurry_home
*   We hurried home as fast as we could. 
    -> END
*   But we decided our trip wasn't done yet.
    *   We immediately left the city. 
        After a few days me returned again.
        -> back_in_almaty
    *   Still, we could not head out just yet. -> fin

== fin
-> END
        
";

        let story = read_story_from_string(content).unwrap();

        for knot in story.knots.values() {
            for stitch in knot.stitches.values() {
                assert!(stitch.root.all_addresses_are_valid());
            }
        }
    }

    #[test]
    fn reading_story_from_string_initializes_all_knot_visit_counts_to_zero() {
        let content = "

== back_in_almaty
-> END

== hurry_home
-> END
= at_home
-> END
        
";

        let story = read_story_from_string(content).unwrap();

        let back_in_almaty = Address::Validated {
            knot: "back_in_almaty".into(),
            stitch: "$ROOT$".into(),
        };
        let hurry_home = Address::Validated {
            knot: "hurry_home".into(),
            stitch: "$ROOT$".into(),
        };
        let at_home = Address::Validated {
            knot: "hurry_home".into(),
            stitch: "at_home".into(),
        };

        assert_eq!(get_num_visited(&back_in_almaty, &story.data).unwrap(), 0);
        assert_eq!(get_num_visited(&hurry_home, &story.data).unwrap(), 0);
        assert_eq!(get_num_visited(&at_home, &story.data).unwrap(), 0);
    }

    #[test]
    fn knots_with_non_default_root_stitch_gets_validated_addresses_that_point_to_them() {
        let content = "
-> almaty

== almaty
= back
We arrived into Almaty at 9.45pm exactly.
-> END
";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();

        assert_eq!(
            &line_buffer[0].text,
            "We arrived into Almaty at 9.45pm exactly.\n"
        );
    }

    #[test]
    fn number_of_visits_in_a_story_is_consistent() {
        let content = "
One
-> root 

== root

+   {visit_twice < 2} -> visit_twice
+   {visit_twice >= 2} {visit_thrice < 3} -> visit_thrice
*   [] -> END

== visit_twice 
Two
-> root

== visit_thrice
Three
-> root

";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();

        let knots = &story.knots;

        let address_root = Address::from_root_knot("root", &knots).unwrap();
        let address_twice = Address::from_root_knot("visit_twice", &knots).unwrap();
        let address_thrice = Address::from_root_knot("visit_thrice", &knots).unwrap();

        assert_eq!(get_num_visited(&address_twice, &story.data).unwrap(), 2);
        assert_eq!(get_num_visited(&address_thrice, &story.data).unwrap(), 3);
        assert_eq!(get_num_visited(&address_root, &story.data).unwrap(), 6);
    }

    #[test]
    fn calling_resume_on_a_story_at_a_choice_returns_the_choice_again() {
        let content = "

== back_in_almaty

After an arduous journey we arrived back in Almaty.

*   We hurried home as fast as we could. 
    -> END
*   But we decided our trip wasn't done yet.
    We immediately left the city. 

";
        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        let choices = story
            .start(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        line_buffer.clear();
        let resume_choices = story
            .resume(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        assert_eq!(choices, resume_choices);
        assert!(line_buffer.is_empty());
    }

    #[test]
    fn after_a_resume_is_made_the_choice_can_be_made_as_usual() {
        let content = "

== back_in_almaty

After an arduous journey we arrived back in Almaty.

*   We hurried home as fast as we could. 
    -> END
*   But we decided our trip wasn't done yet.
    We immediately left the city. 

";
        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story
            .start(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        line_buffer.clear();
        story
            .resume(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();
        story
            .resume(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();
        story
            .resume(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();
        story
            .resume(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        story.resume_with_choice(1, &mut line_buffer).unwrap();

        assert_eq!(line_buffer.len(), 2);
        assert_eq!(
            &line_buffer[0].text,
            "But we decided our trip wasn't done yet.\n"
        );
        assert_eq!(&line_buffer[1].text, "We immediately left the city.\n");
    }

    #[test]
    fn resume_cannot_be_called_on_an_unstarted_story() {
        let content = "

== back_in_almaty
After an arduous journey we arrived back in Almaty.

";
        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        match story.resume(&mut line_buffer) {
            Err(InklingError::ResumeBeforeStart) => (),
            other => panic!(
                "expected `InklingError::ResumeBeforeStart` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn story_can_be_moved_to_another_address() {
        let content = "

We arrived into Almaty at 9.45pm exactly.
-> END

== hurry_home
We hurried home as fast as we could. 
-> END

";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();

        story.move_to("hurry_home", None).unwrap();

        let address = story.stack.last().unwrap();
        assert_eq!(address.get_knot().unwrap(), "hurry_home");
        assert_eq!(address.get_stitch().unwrap(), ROOT_KNOT_NAME);

        line_buffer.clear();
        story.resume(&mut line_buffer).unwrap();

        assert_eq!(
            &line_buffer[0].text,
            "We hurried home as fast as we could.\n"
        );
    }

    #[test]
    fn move_to_addresses_can_include_stitches() {
        let content = "

We arrived into Almaty at 9.45pm exactly.
-> END

== hurry_home
We hurried home as fast as we could. 
-> END

= at_home
Once back home we feasted on cheese.
-> END

";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();

        story.move_to("hurry_home", Some("at_home")).unwrap();

        let address = story.stack.last().unwrap();
        assert_eq!(address.get_knot().unwrap(), "hurry_home");
        assert_eq!(address.get_stitch().unwrap(), "at_home");

        line_buffer.clear();
        story.resume(&mut line_buffer).unwrap();

        assert_eq!(
            &line_buffer[0].text,
            "Once back home we feasted on cheese.\n"
        );
    }

    #[test]
    fn move_to_can_be_called_on_a_story_before_the_start() {
        let content = "

We arrived into Almaty at 9.45pm exactly.
-> END

== hurry_home
We hurried home as fast as we could. 
-> END

= at_home
Once back home we feasted on cheese.
-> END

";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.move_to("hurry_home", Some("at_home")).unwrap();
        story.start(&mut line_buffer).unwrap();

        assert_eq!(
            &line_buffer[0].text,
            "Once back home we feasted on cheese.\n"
        );
    }

    #[test]
    fn move_to_yields_error_if_knot_or_stitch_name_is_invalid() {
        let content = "

We arrived into Almaty at 9.45pm exactly.
-> END

== hurry_home
We hurried home as fast as we could. 
-> END

= at_home
Once back home we feasted on cheese.
-> END

";

        let mut story = read_story_from_string(content).unwrap();

        assert!(story.move_to("fin", None).is_err());
        assert!(story.move_to("hurry_home", Some("not_at_home")).is_err());
    }

    #[test]
    fn resume_with_choice_cannot_be_called_directly_after_a_move() {
        let content = "

We arrived into Almaty at 9.45pm exactly.
*   That was the end of our trip. -> fin

== hurry_home
We hurried home as fast as we could. 

== fin
-> END

";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.start(&mut line_buffer).unwrap();
        story.move_to("hurry_home", None).unwrap();

        match story.resume_with_choice(0, &mut line_buffer) {
            Err(InklingError::ResumeWithoutChoice) => (),
            other => panic!(
                "expected `InklingError::ResumeWithoutChoice` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn get_knot_tags_from_knot_name() {
        let content = "

== tripoli
# country: Libya
# capital
-> END

";

        let story = read_story_from_string(content).unwrap();

        assert_eq!(
            &story.get_knot_tags("tripoli").unwrap(),
            &["country: Libya".to_string(), "capital".to_string()]
        );
    }

    #[test]
    fn getting_knot_tags_with_invalid_name_yields_error() {
        let content = "

== tripoli
# country: Libya
# capital
-> END

";

        let story = read_story_from_string(content).unwrap();

        match story.get_knot_tags("addis_ababa") {
            Err(InklingError::InvalidAddress { knot, stitch }) => {
                assert_eq!(&knot, "addis_ababa");
                assert!(stitch.is_none());
            }
            other => panic!(
                "expected `InklingError::InvalidAddress` but got {:?}",
                other
            ),
        }
    }
}
