//! Structures which contain parsed `Ink` stories and content presented to the user.

use crate::{
    consts::ROOT_KNOT_NAME,
    error::{InklingError, ReadError},
    follow::{ChoiceInfo, EncounteredEvent, FollowData, LineDataBuffer},
    knot::{get_empty_knot_counts, get_mut_stitch, get_num_visited, Address, KnotSet},
    line::Variable,
    log::Logger,
    process::{get_fallback_choices, prepare_choices_for_user, process_buffer},
    story::{
        parse::read_story_content_from_string,
        rng::StoryRng,
        types::{Choice, LineBuffer, Location, Prompt},
        validate::validate_story_content,
    },
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Story with knots, diverts, choices and possibly lots of text.
pub struct Story {
    /// Current address in the story.
    current_address: Address,
    /// Collection of `Knot`s which make up the story.
    pub(crate) knots: KnotSet,
    /// History of visited addresses.
    history: Vec<Address>,
    /// Internal data for the story.
    data: FollowData,
    /// Global tags associated with the story.
    tags: Vec<String>,
    /// Set of last choices presented to the user.
    last_choices: Option<Vec<Choice>>,
    /// Choice that has been set to resume the story with.
    selected_choice: Option<usize>,
    /// Log of warnings and to-do comments encountered when parsing the `Story` from the script.
    pub log: Logger,
}

impl Story {
    /// Resume the story text flow while reading all encountered lines into the supplied buffer.
    ///
    /// Should be called to start the flow through the story or to resume it
    /// after a choice is made with [`make_choice`][crate::story::Story::make_choice()].
    ///
    /// Returns either when the story reaches an end or when a set of choices is encountered,
    /// which requires the user to select one. Make a choice by calling
    /// [`make_choice`][crate::story::Story::make_choice()].
    ///
    /// # Notes
    /// This method does not clear the input `line_buffer` vector before reading more lines
    /// into it. Clearing that buffer has to be done by the caller.
    ///
    /// # Examples
    /// ## Starting the story processing
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
    /// let mut story = read_story_from_string(content).unwrap();
    /// let mut line_buffer = Vec::new();
    ///
    /// story.resume(&mut line_buffer);
    ///
    /// assert_eq!(line_buffer[0].text, "Only in silence the word,\n");
    /// assert_eq!(line_buffer[1].text, "only in dark the light,\n");
    /// assert_eq!(line_buffer[2].text, "only in dying life:\n");
    /// assert_eq!(line_buffer[3].text, "bright the hawk’s flight\n");
    /// assert_eq!(line_buffer[4].text, "on the empty sky.\n");
    /// ```
    ///
    /// ## Making a choice and resuming the flow
    /// ```
    /// # use inkling::read_story_from_string;
    /// let content = "\
    /// The next destination in our strenuous journey was ...
    /// *   Rabat!
    /// *   Addis Ababa!
    /// ";
    ///
    /// // ... setup
    /// # let mut story = read_story_from_string(content).unwrap();
    /// # let mut line_buffer = Vec::new();
    /// #
    /// # story.resume(&mut line_buffer);
    ///
    /// line_buffer.clear();
    /// story.make_choice(0).unwrap();
    /// story.resume(&mut line_buffer).unwrap();
    ///
    /// assert_eq!(&line_buffer[0].text, "Rabat!\n");
    /// ```
    ///
    /// ## Moving to a new knot
    /// ```
    /// # use inkling::{read_story_from_string, Location};
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
    /// # story.resume(&mut line_buffer).unwrap();
    /// let location = Location {
    ///     knot: "mirandas_den".to_string(),
    ///     stitch: Some("meeting".to_string()),
    /// };
    ///
    /// story.move_to(&location).unwrap();
    /// # line_buffer.clear();
    /// story.resume(&mut line_buffer).unwrap();
    ///
    /// assert_eq!(&line_buffer[0].text, "Miranda was waiting in her office.\n");
    /// ```
    pub fn resume(&mut self, line_buffer: &mut LineBuffer) -> Result<Prompt, InklingError> {
        // Break early if we are at a choice but no choice has yet been made
        match (self.selected_choice, self.last_choices.as_ref()) {
            (None, Some(choices)) => return Ok(Prompt::Choice(choices.clone())),
            _ => (),
        }

        let selection = self.selected_choice.take();

        self.follow_story_wrapper(selection, line_buffer)
    }

    /// Make a choice from a given set of options.
    ///
    /// The `selection` index corresponds to the index in the list of choices that was
    /// previously returned when the branching point was reached. This list can be retrieved
    /// again by calling [`resume`][crate::story::Story::resume()] on the story before making
    /// a choice: once a choice has been successfully made, a call to `resume` will continue
    /// the text flow from that branch.
    ///
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
    /// // ... setup
    /// # let mut story = read_story_from_string(content).unwrap();
    /// # let mut line_buffer = Vec::new();
    ///
    /// if let Prompt::Choice(choices) = story.resume(&mut line_buffer).unwrap() {
    ///     story.make_choice(0).unwrap();
    ///     story.resume(&mut line_buffer);
    /// }
    ///
    /// assert_eq!(line_buffer.last().unwrap().text, "“Miao!”\n");
    /// ```
    ///
    /// # Errors
    /// *   [`MadeChoiceWithoutChoice`][crate::error::InklingError::MadeChoiceWithoutChoice]:
    ///     if the story is not currently at a branching point.
    pub fn make_choice(&mut self, selection: usize) -> Result<(), InklingError> {
        let index = self
            .last_choices
            .as_ref()
            .ok_or(InklingError::MadeChoiceWithoutChoice)
            .and_then(|last_choices| {
                last_choices
                    .get(selection)
                    .ok_or(InklingError::InvalidChoice {
                        selection,
                        presented_choices: last_choices.clone(),
                    })
                    .map(|choice| choice.index)
            })?;

        self.selected_choice.replace(index);
        self.last_choices = None;

        Ok(())
    }

    /// Move the story to another knot or stitch.
    ///
    /// A move can be performed at any time, before or after starting the story. It
    /// simply updates the current internal address in the story to the given address.
    /// If no stitch name is given the default stitch from the root will be selected.
    ///
    /// After moving to a new location, call [`resume`][crate::story::Story::resume()]
    /// to continue the text flow from that point.
    ///
    /// # Examples
    /// ## Using `Location` to move
    /// ```
    /// // From ‘Purge’ by Sofi Oksanen
    /// # use inkling::{read_story_from_string, Location};
    /// let content = "\
    /// May, 1949
    /// For the free Estonia!
    ///
    /// === chapter_one ===
    /// 1992, western Estonia
    /// Aliide Truu stared at the fly and the fly stared right back at her.
    /// ";
    ///
    /// // ... setup
    /// # let mut story = read_story_from_string(content).unwrap();
    /// # let mut line_buffer = Vec::new();
    ///
    /// // Let’s skip ahead!
    /// let location = Location {
    ///     knot: "chapter_one".to_string(),
    ///     stitch: None,
    /// };
    ///
    /// story.move_to(&location).unwrap();
    /// story.resume(&mut line_buffer).unwrap();
    ///
    /// assert_eq!(&line_buffer[0].text, "1992, western Estonia\n");
    /// ```
    ///
    /// ## Using `From<&str>` for `Location`
    /// ```
    /// # use inkling::{read_story_from_string, Location};
    /// let content = "\
    /// == 24th_island_sequence
    /// Island sequence 24 is ending.
    /// ";
    ///
    /// let mut story = read_story_from_string(content).unwrap();
    ///
    /// story.move_to(&"24th_island_sequence".into()).unwrap();
    /// assert_eq!(&story.get_current_location(), &Location::from("24th_island_sequence"));
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidAddress`][crate::error::InklingError::InvalidAddress]: if the given
    ///     location does not exist in the story.
    pub fn move_to(&mut self, location: &Location) -> Result<(), InklingError> {
        let to_address = Address::from_location(location, &self.knots).map_err(|_| {
            InklingError::InvalidAddress {
                location: location.clone(),
            }
        })?;

        self.update_last_stack(&to_address);

        self.last_choices = None;
        self.selected_choice = None;

        Ok(())
    }

    #[deprecated(since = "0.12.6", note = "will be removed in 1.0.0")]
    /// Mark the story as being ready to start the text flow processing.
    pub fn start(&mut self) -> Result<(), InklingError> {
        Ok(())
    }

    /// Get the knot and stitch (if applicable) that the story is at currently.
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Location, Prompt};
    /// let content = "\
    /// === gesichts_apartment ===
    /// = dream
    /// Gesicht wakes up from a nightmare. Something horrible is afoot.
    /// ";
    ///
    /// let mut story = read_story_from_string(content).unwrap();
    ///
    /// let location = Location {
    ///     knot: "gesichts_apartment".to_string(),
    ///     stitch: Some("dream".to_string()),
    /// };
    ///
    /// story.move_to(&location).unwrap();
    ///
    /// assert_eq!(story.get_current_location(), location);
    /// ```
    pub fn get_current_location(&self) -> Location {
        let (knot, stitch) = match self.current_address.get_knot_and_stitch() {
            Ok(result) => result,
            Err(_) => {
                eprintln!("`inkling` encountered an error: the current location in the story is a variable, which should not happen");
                (ROOT_KNOT_NAME, ROOT_KNOT_NAME)
            }
        };

        if stitch == ROOT_KNOT_NAME {
            Location::from(knot)
        } else {
            Location::with_stitch(knot, stitch)
        }
    }

    /// Get the tags associated with the given knot.
    ///
    /// Returns `None` if no knot with the given name exists in the story.
    ///
    /// # Examples
    /// ```
    /// # use inkling::read_story_from_string;
    /// // From ‘Sanshirō’ by Natsume Sōseki
    /// let content = "\
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
    pub fn get_knot_tags(&self, knot_name: &str) -> Option<Vec<String>> {
        self.knots.get(knot_name).map(|knot| knot.tags.clone())
    }

    /// Get the number of times a knot or stitch has been visited so far.
    ///
    /// Returns `None` if the given knot and stitch does not exist in the `Story`.
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Location};
    /// # let content = "\
    /// # -> depths
    /// # === depths ===
    /// # You enter the dungeon. Bravely or foolhardily? Who is to decide?
    /// # ";
    /// # let mut story = read_story_from_string(content).unwrap();
    /// # let mut line_buffer = Vec::new();
    /// # story.resume(&mut line_buffer).unwrap();
    /// #
    /// let location = Location {
    ///     knot: "depths".to_string(),
    ///     stitch: None,
    /// };
    ///
    /// # story.move_to(&location).unwrap();
    /// # story.resume(&mut line_buffer).unwrap();
    /// #
    /// let num_visited = story.get_num_visited(&location).unwrap();
    /// assert_eq!(num_visited, 2);
    /// ```
    pub fn get_num_visited(&self, location: &Location) -> Option<u32> {
        let address = Address::from_location(&location, &self.knots).ok()?;

        get_num_visited(&address, &self.data).ok()
    }

    /// Retrieve the global tags associated with the story.
    ///
    /// # Example
    /// ```
    /// # use inkling::read_story_from_string;
    /// let content = "\
    /// ## title: inkling
    /// ## author: Petter Johansson
    /// ";
    ///
    /// let story = read_story_from_string(content).unwrap();
    ///
    /// let tags = story.get_story_tags();
    /// assert_eq!(&tags[0], "title: inkling");
    /// assert_eq!(&tags[1], "author: Petter Johansson");
    /// ```
    pub fn get_story_tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    /// Retrieve the value of a global variable.
    ///
    /// Returns `None` if no variable with the given name exists in the `Story`.
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Variable};
    /// let content = "\
    /// VAR books_in_library = 3
    /// VAR title = \"A Momentuous Spectacle\"
    /// ";
    ///
    /// let story = read_story_from_string(content).unwrap();
    ///
    /// assert_eq!(story.get_variable("books_in_library").unwrap(), Variable::Int(3));
    /// ```
    pub fn get_variable(&self, name: &str) -> Option<Variable> {
        self.data
            .variables
            .get(name)
            .map(|variable_info| variable_info.variable.clone())
    }

    /// Set the value of an existing global variable.
    ///
    /// New variables cannot be created using this method. They have to be defined in the Ink
    /// script file. Constant variables cannot be modified. An error is returned if the given
    /// name corresponds to a constant variable.
    ///
    /// The assignment is type checked: a variable of integer type cannot be changed to
    /// contain a decimal number, a string, or anything else. An error will be returned
    /// if this is attempted.
    ///
    /// Note that this method accepts values which implement `Into<Variable>`. This is implemented
    /// for integers, floating point numbers, booleans and string representations, so those
    /// can be used without a lot of typing.
    ///
    /// # Examples
    /// ## Fully specifying variable type
    /// ```
    /// # use inkling::{read_story_from_string, Variable};
    /// let content = "\
    /// VAR hunted_by_police = false
    /// VAR num_passengers = 0
    /// CONST price_of_ticket = 7.50
    /// ";
    ///
    /// let mut story = read_story_from_string(content).unwrap();
    ///
    /// assert!(story.set_variable("num_passengers", Variable::Int(5)).is_ok());
    /// ```
    ///
    /// ## Inferring type from input
    /// ```
    /// # use inkling::{read_story_from_string, Variable};
    /// # let content = "\
    /// # VAR hunted_by_police = false
    /// # VAR num_passengers = 0
    /// # CONST price_of_ticket = 7.50
    /// # ";
    /// # let mut story = read_story_from_string(content).unwrap();
    /// assert!(story.set_variable("num_passengers", 5).is_ok());
    /// ```
    ///
    /// ## Invalid assignment of different type
    /// ```
    /// # use inkling::{read_story_from_string, Variable};
    /// # let content = "\
    /// # VAR hunted_by_police = false
    /// # VAR num_passengers = 0
    /// # CONST price_of_ticket = 7.50
    /// # ";
    /// # let mut story = read_story_from_string(content).unwrap();
    /// assert!(story.set_variable("hunted_by_police", 10).is_err());
    /// assert!(story.set_variable("hunted_by_police", true).is_ok());
    /// ```
    ///
    /// ## Assignment to constant variable is invalid
    /// ```
    /// # use inkling::{read_story_from_string, Variable};
    /// # let content = "\
    /// # VAR hunted_by_police = false
    /// # VAR num_passengers = 0
    /// # CONST price_of_ticket = 7.50
    /// # ";
    /// # let mut story = read_story_from_string(content).unwrap();
    /// assert!(story.set_variable("price_of_ticket", 1.5).is_err());
    /// ```
    ///
    /// # Errors
    /// *   [`AssignedToConst`][crate::error::InklingError::AssignedToConst]: if the name
    ///     refers to a constant variable.
    /// *   [`InvalidVariable`][crate::error::InklingError::InvalidVariable]: if the name
    ///     does not refer to a global variable that exists in the story.
    /// *   [`VariableError`][crate::error::InklingError::VariableError]: if
    ///     the existing variable has a different type to the input variable.
    pub fn set_variable<T: Into<Variable>>(
        &mut self,
        name: &str,
        value: T,
    ) -> Result<(), InklingError> {
        self.data
            .variables
            .get_mut(name)
            .ok_or(InklingError::InvalidVariable {
                name: name.to_string(),
            })
            .and_then(|variable_info| variable_info.assign(value.into(), name))
    }

    /// Wrapper for calling `follow_story` with a prepared internal buffer.
    ///
    /// Updates the stack to the last visited address and the last presented set of choices
    /// if encountered.
    fn follow_story_wrapper(
        &mut self,
        selection: Option<usize>,
        line_buffer: &mut LineBuffer,
    ) -> Result<Prompt, InklingError> {
        let mut internal_buffer = Vec::new();

        let (result, last_address) = follow_story(
            &self.current_address,
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

                Ok(Prompt::Choice(choices))
            }
            other => Ok(other),
        }
    }

    /// Set the given address as active on the stack.
    fn update_last_stack(&mut self, address: &Address) {
        self.current_address = address.clone();
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
pub fn read_story_from_string(string: &str) -> Result<Story, ReadError> {
    let mut log = Logger::default();
    let (mut knots, variables, tags) = read_story_content_from_string(string, &mut log)?;

    let data = FollowData {
        knot_visit_counts: get_empty_knot_counts(&knots),
        variables,
        rng: StoryRng::default(),
    };

    validate_story_content(&mut knots, &data, &mut log)?;

    let root_address = Address::from_root_knot(ROOT_KNOT_NAME, &knots).expect(
        "After successfully creating all knots, the root knot name that was returned from \
         `read_knots_from_string` is not present in the set of created knots. \
         This simply should not be possible",
    );

    Ok(Story {
        current_address: root_address,
        knots,
        history: Vec::new(),
        data,
        tags,
        last_choices: None,
        selected_choice: None,
        log,
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
    data: &mut FollowData,
) -> Result<Choice, InklingError> {
    get_fallback_choices(choice_set, data).and_then(|choices| {
        choices.first().cloned().ok_or(InklingError::OutOfChoices {
            location: Location::from(current_address.to_string().as_ref()),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        follow::FollowDataBuilder,
        knot::{get_num_visited, increment_num_visited},
        story::parse::tests::read_knots_from_string,
    };

    fn mock_last_choices(choices: &[(&str, usize)]) -> Vec<Choice> {
        choices
            .iter()
            .map(|(text, index)| Choice {
                text: text.to_string(),
                tags: Vec::new(),
                index: *index,
            })
            .collect()
    }

    fn mock_follow_data(knots: &KnotSet) -> FollowData {
        FollowDataBuilder::new()
            .with_knots(get_empty_knot_counts(knots))
            .build()
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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let mut buffer = Vec::new();

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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let mut buffer = Vec::new();

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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let root_address = Address::from_root_knot("select_destination", &knots).unwrap();

        let mut buffer = Vec::new();

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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let root_address = Address::from_root_knot("back_in_london", &knots).unwrap();

        let mut buffer = Vec::new();

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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let done_address = Address::from_root_knot("knot_done", &knots).unwrap();
        let end_address = Address::from_root_knot("knot_end", &knots).unwrap();

        let mut buffer = Vec::new();

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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();
        let divert_address = Address::from_root_knot("tripoli", &knots).unwrap();

        let mut buffer = Vec::new();

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
        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let current_address = Address::from_root_knot("tripoli", &knots).unwrap();

        let mut buffer = Vec::new();

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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();

        let mut line_buffer = Vec::new();

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
            Address::from_parts_unchecked("tripoli", Some("cinema"))
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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let current_address = Address::from_root_knot("addis_ababa", &knots).unwrap();

        let mut line_buffer = Vec::new();

        let (_, last_address) = follow_story(
            &current_address,
            &mut line_buffer,
            None,
            &mut knots,
            &mut data,
        )
        .unwrap();

        assert_eq!(last_address, Address::from_parts_unchecked("tripoli", None));
    }

    /***********************
     * `Story` API testing *
     ***********************/

    #[test]
    fn make_choice_sets_the_choice_index_from_the_last_choices_set() {
        let mut story = read_story_from_string("Content.").unwrap();
        story
            .last_choices
            .replace(mock_last_choices(&[("", 2), ("", 4)]));

        story.make_choice(1).unwrap();

        assert_eq!(story.selected_choice, Some(4));
    }

    #[test]
    fn make_choice_resets_last_choices_vector() {
        let mut story = read_story_from_string("Content.").unwrap();
        story.last_choices.replace(mock_last_choices(&[("", 0)]));

        story.make_choice(0).unwrap();

        assert!(story.last_choices.is_none());
    }

    #[test]
    fn make_choice_yields_an_error_if_a_choice_has_not_been_prompted() {
        let mut story = read_story_from_string("Content.").unwrap();

        match story.make_choice(0) {
            Err(InklingError::MadeChoiceWithoutChoice) => (),
            other => panic!(
                "expected `InklingError::MadeChoiceWithoutChoice` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn make_choice_yields_an_error_if_choice_index_is_not_in_last_choices_set() {
        let mut story = read_story_from_string("Content.").unwrap();

        let last_choices = mock_last_choices(&[("Choice 1", 0), ("Choice 2", 2)]);
        story.last_choices.replace(last_choices.clone());

        match story.make_choice(2) {
            Err(InklingError::InvalidChoice {
                selection,
                presented_choices,
            }) => {
                assert_eq!(selection, 2);
                assert_eq!(presented_choices, last_choices);
            }
            other => panic!("expected `InklingError::InvalidChoice` but got {:?}", other),
        }
    }

    #[test]
    fn calling_resume_continues_the_text_flow_with_the_choice_that_was_made() {
        let content = "
\"To be, or not to be ...\"
*   [To be]
*   [Not to be]
    \"Not to be.\" – Jack Slater
";

        let mut story = read_story_from_string(content).unwrap();
        let mut line_buffer = Vec::new();

        story.resume(&mut line_buffer).unwrap();

        story.make_choice(1).unwrap();

        story.resume(&mut line_buffer).unwrap();

        assert_eq!(&line_buffer[1].text, "\"Not to be.\" – Jack Slater\n");
    }

    #[test]
    fn following_story_wrapper_updates_current_address_to_last_address() {
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
        story.move_to(&"addis_ababa".into()).unwrap();

        let mut line_buffer = Vec::new();

        story.follow_story_wrapper(None, &mut line_buffer).unwrap();

        let address = Address::from_parts_unchecked("tripoli", Some("cinema"));

        assert_eq!(story.current_address, address);
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
        story.move_to(&"knot".into()).unwrap();

        let mut line_buffer = Vec::new();

        let choices = story
            .resume(&mut line_buffer)
            .unwrap()
            .get_choices()
            .unwrap();

        assert_eq!(choices.len(), 1);

        story.make_choice(0).unwrap();
        story.resume(&mut line_buffer).unwrap();

        assert_eq!(&line_buffer[1].text, "Fallback choice\n");
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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let current_address = Address::from_root_knot("first", &knots).unwrap();

        let mut line_buffer = Vec::new();
        let mut internal_buffer = Vec::new();

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

        let mut knots = read_knots_from_string(content).unwrap();

        let mut data = mock_follow_data(&knots);
        validate_story_content(&mut knots, &data, &mut Logger::default()).unwrap();

        let current_address = Address::from_root_knot("tripoli", &knots).unwrap();

        let mut line_buffer = Vec::new();
        let mut internal_buffer = Vec::new();

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
        story.move_to(&"knot".into()).unwrap();

        let mut line_buffer = Vec::new();

        story.resume(&mut line_buffer).unwrap();

        story.make_choice(0).unwrap();

        match story.resume(&mut line_buffer) {
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

        story.move_to(&"knot".into()).unwrap();
        story.resume(&mut line_buffer).unwrap();

        let last_choices = story.last_choices.as_ref().unwrap();

        assert_eq!(last_choices.len(), 2);
        assert_eq!(&last_choices[0].text, "Choice 1");
        assert_eq!(&last_choices[1].text, "Choice 2");
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
        story.move_to(&"back_in_almaty".into()).unwrap();

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

        story.resume(&mut line_buffer).unwrap();

        let address = Address::from_root_knot("$ROOT$", &story.knots).unwrap();

        assert_eq!(get_num_visited(&address, &story.data).unwrap(), 1);
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

        let back_in_almaty = Address::from_parts_unchecked("back_in_almaty", None);
        let hurry_home = Address::from_parts_unchecked("hurry_home", None);
        let at_home = Address::from_parts_unchecked("hurry_home", Some("at_home"));

        assert_eq!(get_num_visited(&back_in_almaty, &story.data).unwrap(), 0);
        assert_eq!(get_num_visited(&hurry_home, &story.data).unwrap(), 0);
        assert_eq!(get_num_visited(&at_home, &story.data).unwrap(), 0);
    }

    #[test]
    fn reading_story_from_string_sets_global_tags() {
        let content = "

# title: inkling
# author: Petter Johansson

";
        let story = read_story_from_string(content).unwrap();

        assert_eq!(
            &story.tags,
            &[
                "title: inkling".to_string(),
                "author: Petter Johansson".to_string()
            ]
        );
    }

    #[test]
    fn reading_story_from_string_sets_global_variables() {
        let content = "

VAR counter = 0
VAR hazardous = true
VAR warning_message = \"ADVARSEL\"

";

        let story = read_story_from_string(content).unwrap();

        let variables = &story.data.variables;
        assert_eq!(variables.len(), 3);

        assert_eq!(variables.get("counter").unwrap().variable, Variable::Int(0));
        assert_eq!(
            variables.get("hazardous").unwrap().variable,
            Variable::Bool(true)
        );

        assert_eq!(
            variables.get("warning_message").unwrap().variable,
            Variable::String("ADVARSEL".to_string())
        );
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

        story.resume(&mut line_buffer).unwrap();

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

        story.resume(&mut line_buffer).unwrap();

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
        story.move_to(&"back_in_almaty".into()).unwrap();

        let mut line_buffer = Vec::new();

        let choices = story
            .resume(&mut line_buffer)
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
    fn calling_resume_on_a_story_at_a_choice_without_making_a_choice_does_not_update_the_story() {
        let content = "

== back_in_almaty

After an arduous journey we arrived back in {Almaty|Addis Ababa|Tripoli}.

*   We hurried home {as fast as we could|slowly}.
    -> END
*   But we decided our trip wasn't done yet.
    We immediately left the city.

";
        let mut story = read_story_from_string(content).unwrap();
        story.move_to(&"back_in_almaty".into()).unwrap();

        let mut line_buffer = Vec::new();

        story.resume(&mut line_buffer).unwrap();

        let story_backup = story.clone();
        story.resume(&mut line_buffer).unwrap();

        assert_eq!(story, story_backup);
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

        story.move_to(&"hurry_home".into()).unwrap();

        let address = story.current_address.clone();
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
    fn moving_to_a_new_location_resets_current_choice_list() {
        let content = "

== back_in_almaty

After an arduous journey we arrived back in {Almaty|Addis Ababa|Tripoli}.

*   We hurried home {as fast as we could|slowly}.
    -> END
*   But we decided our trip wasn't done yet.
    We immediately left the city.

== hurry_home
We hurried home as fast as we could.
-> END
";
        let mut story = read_story_from_string(content).unwrap();
        story.move_to(&"back_in_almaty".into()).unwrap();

        let mut line_buffer = Vec::new();
        story.resume(&mut line_buffer).unwrap();

        story.move_to(&"hurry_home".into()).unwrap();

        assert!(story.last_choices.is_none());
    }

    #[test]
    fn moving_to_a_new_location_resets_selected_choice() {
        let content = "

== back_in_almaty

After an arduous journey we arrived back in {Almaty|Addis Ababa|Tripoli}.

*   We hurried home {as fast as we could|slowly}.
    -> END
*   But we decided our trip wasn't done yet.
    We immediately left the city.

== hurry_home
We hurried home as fast as we could.
-> END
";
        let mut story = read_story_from_string(content).unwrap();
        story.move_to(&"back_in_almaty".into()).unwrap();

        let mut line_buffer = Vec::new();
        story.resume(&mut line_buffer).unwrap();
        story.make_choice(0).unwrap();

        story.move_to(&"hurry_home".into()).unwrap();

        assert!(story.selected_choice.is_none());
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

        let location = Location::with_stitch("hurry_home", "at_home");
        story.move_to(&location).unwrap();

        let address = story.current_address.clone();
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

        assert!(story.move_to(&"fin".into()).is_err());

        let location = Location::with_stitch("hurry_home", "not_at_home");
        assert!(story.move_to(&location).is_err());
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
    fn getting_knot_tags_with_invalid_name_yields_none() {
        let content = "

== tripoli
# country: Libya
# capital
-> END

";

        let story = read_story_from_string(content).unwrap();
        assert!(story.get_knot_tags("addis_ababa").is_none());
    }

    #[test]
    fn current_location_in_story_is_the_current_address() {
        let content = "

We arrived into Almaty at 9.45pm exactly.
-> END

== hurry_home
= at_home
We hurried home as fast as we could.
-> END

";

        let mut story = read_story_from_string(content).unwrap();

        assert_eq!(
            story.get_current_location(),
            Location::from(ROOT_KNOT_NAME),
            // (ROOT_KNOT_NAME.to_string(), None)
        );

        story.move_to(&"hurry_home".into()).unwrap();

        assert_eq!(
            story.get_current_location(),
            Location::with_stitch("hurry_home", "at_home"),
            // ("hurry_home".to_string(), Some("at_home".to_string()))
        );
    }

    #[test]
    fn getting_number_of_visits_uses_data() {
        let content = "
== hurry_home
We hurried home as fast as we could.
-> END

= at_home
Once back home we feasted on cheese.
-> END

";

        let mut story = read_story_from_string(content).unwrap();

        let location = Location::with_stitch("hurry_home", "at_home");
        let address = Address::from_location(&location, &story.knots).unwrap();

        increment_num_visited(&address, &mut story.data).unwrap();
        increment_num_visited(&address, &mut story.data).unwrap();

        assert_eq!(story.get_num_visited(&"hurry_home".into()).unwrap(), 0);
        assert_eq!(story.get_num_visited(&location).unwrap(), 2);
    }

    #[test]
    fn getting_number_of_visits_yields_none_if_knot_or_stitch_name_is_invalid() {
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

        let story = read_story_from_string(content).unwrap();

        assert!(story.get_num_visited(&"fin".into()).is_none());
        assert!(story
            .get_num_visited(&Location::with_stitch("hurry_home", "fin"))
            .is_none());
    }

    #[test]
    fn getting_variable_returns_cloned() {
        let content = "

VAR hazardous = true

";

        let story = read_story_from_string(content).unwrap();

        assert_eq!(
            story.get_variable("hazardous").unwrap(),
            Variable::Bool(true)
        );
    }

    #[test]
    fn setting_variable_is_only_allowed_without_changing_type() {
        let content = "

VAR counter = 3

";

        let mut story = read_story_from_string(content).unwrap();

        story.set_variable("counter", Variable::Int(5)).unwrap();
        assert_eq!(
            story.data.variables.get("counter").unwrap().variable,
            Variable::Int(5)
        );

        assert!(story.set_variable("counter", Variable::Float(5.0)).is_err());
        assert!(story.set_variable("counter", Variable::Bool(true)).is_err());
    }

    #[test]
    fn setting_variable_is_only_allowed_for_non_const_variables() {
        let content = "

VAR non_const_variable = 3
CONST const_variable = 3

";

        let mut story = read_story_from_string(content).unwrap();

        assert!(story
            .set_variable("non_const_variable", Variable::Int(5))
            .is_ok());

        let err = story
            .set_variable("const_variable", Variable::Int(5))
            .unwrap_err();
        let expected_err = InklingError::AssignedToConst {
            name: "const_variable".to_string(),
        };

        assert_eq!(format!("{:?}", err), format!("{:?}", expected_err));
    }

    #[test]
    fn setting_variable_can_infer_number_boolean_and_string_types() {
        let content = "

VAR hazardous = false
VAR counter = 3
VAR precision = 1.23
VAR message = \"boring text\"

";

        let mut story = read_story_from_string(content).unwrap();

        assert!(story.set_variable("hazardous", true).is_ok());
        assert!(story.set_variable("counter", -10).is_ok());
        assert!(story.set_variable("precision", 5.45).is_ok());
        assert!(story
            .set_variable("message", "What a pleasure to see you!")
            .is_ok());

        assert_eq!(
            story.data.variables.get("counter").unwrap().variable,
            Variable::Int(-10)
        );

        assert_eq!(
            story.data.variables.get("hazardous").unwrap().variable,
            Variable::Bool(true)
        );

        assert_eq!(
            story.data.variables.get("precision").unwrap().variable,
            Variable::Float(5.45)
        );

        assert_eq!(
            story.data.variables.get("message").unwrap().variable,
            Variable::String("What a pleasure to see you!".to_string())
        );
    }

    #[test]
    fn global_tags_can_be_retrieved() {
        let content = "

# title: inkling
# author: Petter Johansson

";
        let story = read_story_from_string(content).unwrap();

        assert_eq!(
            &story.get_story_tags(),
            &[
                "title: inkling".to_string(),
                "author: Petter Johansson".to_string()
            ]
        );
    }
}
