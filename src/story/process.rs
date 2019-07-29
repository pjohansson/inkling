//! Process content to display to the user.

use crate::{
    error::{InklingError, InternalError},
    follow::{ChoiceInfo, LineDataBuffer, LineText},
    line::{Condition, InternalLine},
};

use super::{
    address::Address,
    story::{get_stitch, Choice, Knots, Line, LineBuffer},
};

use std::{cell::RefCell, rc::Rc};

/// Process internal lines to a user-ready state.
pub fn process_buffer(into_buffer: &mut LineBuffer, from_buffer: LineDataBuffer) {
    let mut iter = from_buffer
        .into_iter()
        .filter(|line| !line.text.trim().is_empty())
        .peekable();

    while let Some(mut line) = iter.next() {
        add_line_ending(&mut line, iter.peek());

        into_buffer.push(Line {
            text: line.text,
            tags: line.tags,
        });
    }
}

/// Prepare a list of choices to display to the user.
///
/// Preserve line tags in case processing is desired. Choices are filtered
/// based on a set condition (currently: visited or not, unless sticky).
pub fn prepare_choices_for_user(
    choices: &[ChoiceInfo],
    current_address: &Address,
    knots: &Knots,
) -> Result<Vec<Choice>, InklingError> {
    get_available_choices(choices, current_address, knots, false)
}

/// Prepare a list of fallback choices from the given set.
///
/// From this set the first item should be automatically followed by the story. This,
/// however, is the caller's responsibility.
pub fn get_fallback_choices(
    choices: &[ChoiceInfo],
    current_address: &Address,
    knots: &Knots,
) -> Result<Vec<Choice>, InklingError> {
    get_available_choices(choices, current_address, knots, true)
}

/// Return the currently available choices in the set.
///
/// Filters choices which do not fulfil the conditions to be active. These can for example
/// be due to a non-sticky choice having been previously selected or due to some other
/// condition not being met.
///
/// If the `fallback` variable is true, return only the fallback choices which meet
/// the criteria. Otherwise return only non-fallback choices.
fn get_available_choices(
    choices: &[ChoiceInfo],
    current_address: &Address,
    knots: &Knots,
    fallback: bool,
) -> Result<Vec<Choice>, InklingError> {
    let choices_with_filter_values =
        zip_choices_with_filter_values(choices, current_address, knots, fallback)?;

    let filtered_choices = choices_with_filter_values
        .into_iter()
        .filter_map(|(keep, choice)| if keep { Some(choice) } else { None })
        .collect();

    Ok(filtered_choices)
}

/// Pair every choice with whether it fulfils its conditions.
fn zip_choices_with_filter_values(
    choices: &[ChoiceInfo],
    current_address: &Address,
    knots: &Knots,
    fallback: bool,
) -> Result<Vec<(bool, Choice)>, InklingError> {
    let checked_choices = check_choices_for_conditions(choices, current_address, knots, fallback)?;

    choices
        .iter()
        .zip(checked_choices.into_iter())
        .enumerate()
        .map(|(i, (ChoiceInfo { choice_data, .. }, keep))| {
            let (text, tags) = if keep {
                process_choice_text_and_tags(choice_data.selection_text.clone())
            } else {
                // If we are filtering the choice we do not want it's processed selection
                // text to update their state. Instead, we clone the data and process that.

                let independent_text = choice_data.selection_text.borrow().clone();
                process_choice_text_and_tags(Rc::new(RefCell::new(independent_text)))
            }?;

            Ok((
                keep,
                Choice {
                    text,
                    tags,
                    index: i,
                },
            ))
        })
        .collect()
}

/// Process a line into a string and return it with its tags.
fn process_choice_text_and_tags(
    choice_line: Rc<RefCell<InternalLine>>,
) -> Result<(String, Vec<String>), InklingError> {
    let mut data_buffer = Vec::new();

    let mut line = choice_line.borrow_mut();

    line.process(&mut data_buffer)
        .map_err(|err| InternalError::from(err))?;

    let mut buffer = String::new();

    for data in data_buffer.into_iter() {
        buffer.push_str(&data.text);
    }

    Ok((buffer.trim().to_string(), line.tags.clone()))
}

/// Return a list of whether choices fulfil their conditions.
fn check_choices_for_conditions(
    choices: &[ChoiceInfo],
    current_address: &Address,
    knots: &Knots,
    keep_only_fallback: bool,
) -> Result<Vec<bool>, InklingError> {
    let mut checked_conditions = Vec::new();

    for ChoiceInfo {
        num_visited,
        choice_data,
    } in choices.iter()
    {
        let mut keep = true;

        for condition in choice_data.conditions.iter() {
            keep = check_condition(condition, current_address, knots)?;

            if !keep {
                break;
            }
        }

        keep = keep
            && (choice_data.is_sticky || *num_visited == 0)
            && (choice_data.is_fallback == keep_only_fallback);

        checked_conditions.push(keep);
    }

    Ok(checked_conditions)
}

/// Add a newline character to the current line if it is not glued to the next.
///
/// Ensure that only a single whitespace remains between the lines if they are glued.
fn add_line_ending(line: &mut LineText, next_line: Option<&LineText>) {
    let glue = next_line
        .map(|next_line| line.glue_end || next_line.glue_begin)
        .unwrap_or(false);

    let whitespace = glue && {
        next_line
            .map(|next_line| line.text.ends_with(' ') || next_line.text.starts_with(' '))
            .unwrap_or(false)
    };

    if !glue || whitespace {
        let mut text = line.text.trim().to_string();

        if whitespace {
            text.push(' ');
        }

        if !glue {
            text.push('\n');
        }

        line.text = text;
    }
}

/// Check whether a single choice fulfils its conditions.
fn check_condition(
    condition: &Condition,
    current_address: &Address,
    knots: &Knots,
) -> Result<bool, InklingError> {
    match condition {
        Condition::NumVisits {
            address,
            rhs_value,
            ordering,
            not,
        } => {
            let num_visits = get_stitch(address, knots)?.num_visited as i32;

            let value = num_visits.cmp(rhs_value) == *ordering;

            if *not {
                Ok(!value)
            } else {
                Ok(value)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        consts::ROOT_KNOT_NAME,
        follow::LineTextBuilder,
        knot::{Knot, Stitch},
        line::{
            AlternativeBuilder, InternalChoice, InternalChoiceBuilder, InternalLineBuilder,
            LineChunkBuilder,
        },
    };

    use std::{cmp::Ordering, collections::HashMap, str::FromStr};

    fn get_mock_address_and_knots() -> (Address, Knots) {
        let empty_hash_map = HashMap::new();
        let empty_address = Address::Validated {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        (empty_address, empty_hash_map)
    }

    fn create_choice_extra(num_visited: u32, choice_data: InternalChoice) -> ChoiceInfo {
        ChoiceInfo {
            num_visited,
            choice_data,
        }
    }

    #[test]
    fn check_some_conditions_against_number_of_visits_in_a_hash_map() {
        let name = "knot_name".to_string();

        let mut stitch = Stitch::from_str("").unwrap();
        stitch.num_visited = 3;

        let mut stitches = HashMap::new();
        stitches.insert(ROOT_KNOT_NAME.to_string(), stitch);

        let mut knots = HashMap::new();
        knots.insert(
            name.clone(),
            Knot {
                default_stitch: ROOT_KNOT_NAME.to_string(),
                stitches,
            },
        );

        let current_address = Address::from_root_knot("knot_name", &knots).unwrap();

        let greater_than_condition = Condition::NumVisits {
            address: Address::Raw(name.clone()),
            rhs_value: 2,
            ordering: Ordering::Greater,
            not: false,
        };

        assert!(check_condition(&greater_than_condition, &current_address, &knots).unwrap());

        let less_than_condition = Condition::NumVisits {
            address: Address::Raw(name.clone()),
            rhs_value: 2,
            ordering: Ordering::Less,
            not: false,
        };

        assert!(!check_condition(&less_than_condition, &current_address, &knots).unwrap());

        let equal_condition = Condition::NumVisits {
            address: Address::Raw(name.clone()),
            rhs_value: 3,
            ordering: Ordering::Equal,
            not: false,
        };

        assert!(check_condition(&equal_condition, &current_address, &knots).unwrap());

        let not_equal_condition = Condition::NumVisits {
            address: Address::Raw(name.clone()),
            rhs_value: 3,
            ordering: Ordering::Equal,
            not: true,
        };

        assert!(!check_condition(&not_equal_condition, &current_address, &knots).unwrap());
    }

    #[test]
    fn if_condition_checks_knot_that_is_not_in_map_an_error_is_raised() {
        let knots = HashMap::new();

        let gt_condition = Condition::NumVisits {
            address: Address::Raw("knot_name".to_string()),
            rhs_value: 0,
            ordering: Ordering::Greater,
            not: false,
        };

        let current_address = Address::Validated {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        assert!(check_condition(&gt_condition, &current_address, &knots).is_err());
    }

    #[test]
    fn processing_line_buffer_removes_empty_lines() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).build(),
            LineTextBuilder::from_string("").build(),
            LineTextBuilder::from_string(text).build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].text.trim(), text);
        assert_eq!(processed[1].text.trim(), text);
    }

    #[test]
    fn processing_line_buffer_trims_extra_whitespace() {
        let buffer = vec![
            LineTextBuilder::from_string("    Hello, World!    ").build(),
            LineTextBuilder::from_string("    Hello right back at you!  ").build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed.len(), 2);
        assert_eq!(processed[0].text.trim(), "Hello, World!");
        assert_eq!(processed[1].text.trim(), "Hello right back at you!");
    }

    #[test]
    fn processing_line_buffer_adds_newlines_if_no_glue() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).build(),
            LineTextBuilder::from_string(text).build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_removes_newlines_between_lines_with_glue_end_on_first() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).with_glue_end().build(),
            LineTextBuilder::from_string(text).build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_removes_newlines_between_lines_with_glue_start_on_second() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).build(),
            LineTextBuilder::from_string(text).with_glue_begin().build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_with_glue_works_across_empty_lines() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            LineTextBuilder::from_string(text).build(),
            LineTextBuilder::from_string("").build(),
            LineTextBuilder::from_string(text).with_glue_begin().build(),
        ];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_sets_newline_on_last_line_regardless_of_glue() {
        let line = LineTextBuilder::from_string("Mr. and Mrs. Doubtfire")
            .with_glue_end()
            .build();

        let buffer = vec![line];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_keeps_single_whitespace_between_lines_with_glue() {
        let line1 = LineTextBuilder::from_string("Ends with whitespace before glue, ")
            .with_glue_end()
            .build();
        let line2 = LineTextBuilder::from_string(" starts with whitespace after glue")
            .with_glue_begin()
            .build();

        let buffer = vec![line1, line2];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with(' '));
        assert!(!processed[1].text.starts_with(' '));
    }

    #[test]
    fn processing_line_buffer_preserves_tags() {
        let text = "Mr. and Mrs. Doubtfire";
        let tags = vec!["tag 1".to_string(), "tag 2".to_string()];

        let line = LineTextBuilder::from_string(text).with_tags(&tags).build();

        let buffer = vec![line];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed[0].tags, tags);
    }

    #[test]
    fn preparing_choices_returns_selection_text_lines() {
        let choice1 = InternalChoiceBuilder::from_selection_string("Choice 1").build();

        let choice2 = InternalChoiceBuilder::from_selection_string("Choice 2").build();

        let choices = vec![
            create_choice_extra(0, choice1),
            create_choice_extra(0, choice2),
        ];

        let (empty_address, empty_hash_map) = get_mock_address_and_knots();
        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Choice 1");
        assert_eq!(&displayed_choices[1].text, "Choice 2");
    }

    #[test]
    fn preparing_choices_preserves_tags() {
        let tags = vec!["tag 1".to_string(), "tag 2".to_string()];
        let choice = InternalChoiceBuilder::from_string("Choice with tags")
            .with_tags(&tags)
            .build();

        let choices = vec![create_choice_extra(0, choice)];

        let (empty_address, empty_hash_map) = get_mock_address_and_knots();
        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices[0].tags, tags);
    }

    #[test]
    fn processing_choices_checks_conditions() {
        let name = "knot_name".to_string();

        let mut stitch = Stitch::from_str("").unwrap();
        stitch.num_visited = 1;

        let mut stitches = HashMap::new();
        stitches.insert(ROOT_KNOT_NAME.to_string(), stitch);

        let mut knots = HashMap::new();
        knots.insert(
            name.clone(),
            Knot {
                default_stitch: ROOT_KNOT_NAME.to_string(),
                stitches,
            },
        );

        let current_address = Address::from_root_knot("knot_name", &knots).unwrap();

        let fulfilled_condition = Condition::NumVisits {
            address: Address::Raw(name.clone()),
            rhs_value: 0,
            ordering: Ordering::Greater,
            not: false,
        };

        let unfulfilled_condition = Condition::NumVisits {
            address: Address::Raw(name.clone()),
            rhs_value: 2,
            ordering: Ordering::Greater,
            not: false,
        };

        let choice1 = InternalChoiceBuilder::from_string("Removed")
            .with_condition(&unfulfilled_condition)
            .build();
        let choice2 = InternalChoiceBuilder::from_string("Kept")
            .with_condition(&fulfilled_condition)
            .build();
        let choice3 = InternalChoiceBuilder::from_string("Removed")
            .with_condition(&unfulfilled_condition)
            .build();

        let choices = vec![
            create_choice_extra(0, choice1),
            create_choice_extra(0, choice2),
            create_choice_extra(0, choice3),
        ];

        let displayed_choices =
            prepare_choices_for_user(&choices, &current_address, &knots).unwrap();

        assert_eq!(displayed_choices.len(), 1);
        assert_eq!(&displayed_choices[0].text, "Kept");
    }

    #[test]
    fn preparing_choices_filters_choices_which_have_been_visited_for_non_sticky_lines() {
        let choice1 = InternalChoiceBuilder::from_string("Kept").build();
        let choice2 = InternalChoiceBuilder::from_string("Removed").build();
        let choice3 = InternalChoiceBuilder::from_string("Kept").build();

        let choices = vec![
            create_choice_extra(0, choice1),
            create_choice_extra(1, choice2),
            create_choice_extra(0, choice3),
        ];

        let (empty_address, empty_hash_map) = get_mock_address_and_knots();
        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Kept");
        assert_eq!(&displayed_choices[1].text, "Kept");
    }

    #[test]
    fn preparing_choices_does_not_filter_visited_sticky_lines() {
        let choice1 = InternalChoiceBuilder::from_string("Kept").build();
        let choice2 = InternalChoiceBuilder::from_string("Removed").build();
        let choice3 = InternalChoiceBuilder::from_string("Kept")
            .is_sticky()
            .build();

        let choices = vec![
            create_choice_extra(0, choice1),
            create_choice_extra(1, choice2),
            create_choice_extra(1, choice3),
        ];

        let (empty_address, empty_hash_map) = get_mock_address_and_knots();
        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Kept");
        assert_eq!(&displayed_choices[1].text, "Kept");
    }

    #[test]
    fn preparing_choices_filters_fallback_choices() {
        let choice1 = InternalChoiceBuilder::from_string("Kept").build();
        let choice2 = InternalChoiceBuilder::from_string("Removed")
            .is_fallback()
            .build();
        let choice3 = InternalChoiceBuilder::from_string("Kept")
            .is_sticky()
            .build();

        let choices = vec![
            create_choice_extra(0, choice1),
            create_choice_extra(0, choice2),
            create_choice_extra(0, choice3),
        ];

        let (empty_address, empty_hash_map) = get_mock_address_and_knots();
        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Kept");
        assert_eq!(&displayed_choices[1].text, "Kept");
    }

    #[test]
    fn fallback_choices_are_filtered_as_usual_choices() {
        let choice1 = InternalChoiceBuilder::from_string("Kept")
            .is_fallback()
            .build();
        let choice2 = InternalChoiceBuilder::from_string("Removed")
            .is_fallback()
            .build();
        let choice3 = InternalChoiceBuilder::from_string("Kept")
            .is_sticky()
            .is_fallback()
            .build();

        let choices = vec![
            create_choice_extra(0, choice1),
            create_choice_extra(1, choice2),
            create_choice_extra(1, choice3),
        ];

        let (empty_address, empty_hash_map) = get_mock_address_and_knots();
        let fallback_choices =
            get_fallback_choices(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(fallback_choices.len(), 2);
        assert_eq!(&fallback_choices[0].text, "Kept");
        assert_eq!(&fallback_choices[1].text, "Kept");
    }

    #[test]
    fn getting_available_choices_processes_the_text() {
        let alternative = AlternativeBuilder::cycle()
            .with_line(LineChunkBuilder::from_string("once").build())
            .with_line(LineChunkBuilder::from_string("twice").build())
            .build();

        let chunk = LineChunkBuilder::new()
            .with_text("Hello ")
            .with_alternative(alternative)
            .with_text("!")
            .build();

        let line = InternalLineBuilder::from_chunk(chunk).build();

        let choice = InternalChoiceBuilder::from_line(line).build();

        let choices = vec![create_choice_extra(0, choice)];

        let (empty_address, empty_hash_map) = get_mock_address_and_knots();

        let presented_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(presented_choices.len(), 1);
        assert_eq!(&presented_choices[0].text, "Hello once!");

        let presented_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(presented_choices.len(), 1);
        assert_eq!(&presented_choices[0].text, "Hello twice!");
    }
}
