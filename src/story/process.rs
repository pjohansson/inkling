//! Process lines to their final form, which will be displayed to the user.

use crate::{
    error::InklingError,
    follow::LineDataBuffer,
    follow::*,
    line::{Condition, *},
};

use super::{
    address::Address,
    story::{get_stitch, Choice, Knots, Line, LineBuffer},
};

/// Process full `LineData` lines to their final state: remove empty lines, add newlines
/// unless glue is present.
pub fn process_buffer(into_buffer: &mut LineBuffer, from_buffer: LineDataBuffer) {
    let mut iter = from_buffer
        .into_iter()
        .filter(|line| !line.text().trim().is_empty())
        .peekable();

    while let Some(mut line) = iter.next() {
        add_line_ending(&mut line, iter.peek());

        into_buffer.push(Line {
            text: line.text(),
            tags: line.tags,
        });
    }
}

/// Prepared the choices with the text that will be displayed to the user.
/// Preserve line tags in case processing is desired. Choices are filtered
/// based on a set condition (currently: visited or not, unless sticky).
pub fn prepare_choices_for_user(
    choices: &[ChoiceExtra],
    current_address: &Address,
    knots: &Knots,
) -> Result<Vec<Choice>, InklingError> {
    get_available_choices(choices, current_address, knots, false)
}

/// Prepare a list of fallback choices from the given set. The first choice will be
/// automatically selected.
pub fn get_fallback_choices(
    choices: &[ChoiceExtra],
    current_address: &Address,
    knots: &Knots,
) -> Result<Vec<Choice>, InklingError> {
    get_available_choices(choices, current_address, knots, true)
}

fn get_available_choices(
    choices: &[ChoiceExtra],
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

fn zip_choices_with_filter_values(
    choices: &[ChoiceExtra],
    current_address: &Address,
    knots: &Knots,
    fallback: bool,
) -> Result<Vec<(bool, Choice)>, InklingError> {
    let checked_choices = check_choices_for_conditions(choices, current_address, knots, fallback)?;

    let filtered_choices = choices
        .iter()
        .enumerate()
        .map(|(i, ChoiceExtra { choice_data, .. })| Choice {
            text: choice_data.selection_text.text().trim().to_string(),
            tags: choice_data.selection_text.tags.clone(),
            index: i,
        })
        .zip(checked_choices.into_iter())
        .map(|(choice, keep)| (keep, choice))
        .collect();

    Ok(filtered_choices)
}

fn check_choices_for_conditions(
    choices: &[ChoiceExtra],
    current_address: &Address,
    knots: &Knots,
    keep_only_fallback: bool,
) -> Result<Vec<bool>, InklingError> {
    let mut checked_conditions = Vec::new();

    for ChoiceExtra {
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

/// Add a newline character if the line is not glued to the next. Retain only a single
/// whitespace between the lines if they are glued.
fn add_line_ending(line: &mut FullLine, next_line: Option<&FullLine>) {
    let glue = next_line
        .map(|next_line| line.glue_end || next_line.glue_begin)
        .unwrap_or(false);

    let whitespace = glue && {
        next_line
            .map(|next_line| line.text().ends_with(' ') || next_line.text().starts_with(' '))
            .unwrap_or(false)
    };

    if !glue || whitespace {
        let mut text = line.text().trim().to_string();

        if whitespace {
            text.push(' ');
        }

        if !glue {
            text.push('\n');
        }

        match line.chunk.items[0] {
            Content::PureText(ref mut content) => *content = text,
            _ => unreachable!(),
        }
    }
}

fn check_condition(
    condition: &Condition,
    current_address: &Address,
    knots: &Knots,
) -> Result<bool, InklingError> {
    match condition {
        Condition::NumVisits {
            name,
            rhs_value,
            ordering,
            not,
        } => {
            let address = Address::from_target_address(name, current_address, knots)?;
            let num_visits = get_stitch(&address, knots)?.num_visited as i32;

            let value = num_visits.cmp(rhs_value) == *ordering;

            if *not {
                Ok(!value)
            } else {
                Ok(value)
            }
        }
    }
}

/// If the story was followed with an invalid choice we want to collect as much information
/// about it as possible. This is done when first encountering the error as the stack
/// is followed, which fills in which `ChoiceData` values were available and which index
/// was used to select with.
///
/// This function fills in the rest of the stub.
pub fn fill_in_invalid_error(
    error_stub: InklingError,
    made_choice: &Choice,
    current_address: &Address,
    knots: &Knots,
) -> InklingError {
    match error_stub {
        InklingError::InvalidChoice {
            index,
            internal_choices,
            ..
        } => {
            let presented_choices =
                zip_choices_with_filter_values(&internal_choices, current_address, knots, false)
                    .unwrap_or(Vec::new());

            InklingError::InvalidChoice {
                index,
                choice: Some(made_choice.clone()),
                internal_choices,
                presented_choices,
            }
        }
        _ => error_stub,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        consts::ROOT_KNOT_NAME,
        knot::{Knot, Stitch},
    };

    use std::{cmp::Ordering, collections::HashMap, str::FromStr};

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
            name: name.clone(),
            rhs_value: 2,
            ordering: Ordering::Greater,
            not: false,
        };

        assert!(check_condition(&greater_than_condition, &current_address, &knots).unwrap());

        let less_than_condition = Condition::NumVisits {
            name: name.clone(),
            rhs_value: 2,
            ordering: Ordering::Less,
            not: false,
        };

        assert!(!check_condition(&less_than_condition, &current_address, &knots).unwrap());

        let equal_condition = Condition::NumVisits {
            name: name.clone(),
            rhs_value: 3,
            ordering: Ordering::Equal,
            not: false,
        };

        assert!(check_condition(&equal_condition, &current_address, &knots).unwrap());

        let not_equal_condition = Condition::NumVisits {
            name: name.clone(),
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
            name: "knot_name".to_string(),
            rhs_value: 0,
            ordering: Ordering::Greater,
            not: false,
        };

        let current_address = Address {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        assert!(check_condition(&gt_condition, &current_address, &knots).is_err());
    }

    #[test]
    fn processing_line_buffer_removes_empty_lines() {
        let text = "Mr. and Mrs. Doubtfire";

        let buffer = vec![
            parse_line(text).unwrap(),
            parse_line("").unwrap(),
            parse_line(text).unwrap(),
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
            parse_line("    Hello, World!    ").unwrap(),
            parse_line("    Hello right back at you!  ").unwrap(),
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

        let buffer = vec![parse_line(text).unwrap(), parse_line(text).unwrap()];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_removes_newlines_between_lines_with_glue_end_on_first() {
        let text = "Mr. and Mrs. Doubtfire";

        let mut line1 = parse_line(text).unwrap();
        line1.glue_end = true;

        let buffer = vec![line1, parse_line(text).unwrap()];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_removes_newlines_between_lines_with_glue_start_on_second() {
        let text = "Mr. and Mrs. Doubtfire";

        let mut line2 = parse_line(text).unwrap();
        line2.glue_begin = true;

        let buffer = vec![parse_line(text).unwrap(), line2];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_with_glue_works_across_empty_lines() {
        let text = "Mr. and Mrs. Doubtfire";

        let mut line3 = parse_line(text).unwrap();
        line3.glue_begin = true;

        let buffer = vec![parse_line(text).unwrap(), parse_line("").unwrap(), line3];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(!processed[0].text.ends_with('\n'));
        assert!(processed[1].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_sets_newline_on_last_line_regardless_of_glue() {
        let text = "Mr. and Mrs. Doubtfire";

        let mut line = parse_line(text).unwrap();
        line.glue_end = true;

        let buffer = vec![line];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert!(processed[0].text.ends_with('\n'));
    }

    #[test]
    fn processing_line_buffer_keeps_single_whitespace_between_lines_with_glue() {
        let mut line1 = parse_line("Ends with whitespace before glue, ").unwrap();
        line1.glue_end = true;

        let mut line2 = parse_line(" starts with whitespace after glue").unwrap();
        line2.glue_begin = true;

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

        let mut line = parse_line(text).unwrap();
        line.tags = tags.clone();

        let buffer = vec![line];

        let mut processed = Vec::new();
        process_buffer(&mut processed, buffer);

        assert_eq!(processed[0].tags, tags);
    }

    #[test]
    fn preparing_choices_returns_selection_text_lines() {
        let empty_text = parse_line("").unwrap();
        let selection_text1 = parse_line("Choice 1").unwrap();
        let selection_text2 = parse_line("Choice 2").unwrap();

        let choice1 = FullChoiceBuilder::from_line(selection_text1)
            .with_display_text(empty_text.clone())
            .build();

        let choice2 = FullChoiceBuilder::from_line(selection_text2)
            .with_display_text(empty_text.clone())
            .build();

        let choices = vec![
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice1,
            },
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice2,
            },
        ];

        let empty_hash_map = HashMap::new();
        let empty_address = Address {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Choice 1");
        assert_eq!(&displayed_choices[1].text, "Choice 2");
    }

    #[test]
    fn preparing_choices_preserves_tags() {
        let tags = vec!["tag 1".to_string(), "tag 2".to_string()];
        let choice = FullChoice::from_string("Choice with tags # tag 1 # tag 2");

        let choices = vec![ChoiceExtra {
            num_visited: 0,
            choice_data: choice,
        }];

        let empty_hash_map = HashMap::new();
        let empty_address = Address {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

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
            name: name.clone(),
            rhs_value: 0,
            ordering: Ordering::Greater,
            not: false,
        };

        let unfulfilled_condition = Condition::NumVisits {
            name: name.clone(),
            rhs_value: 2,
            ordering: Ordering::Greater,
            not: false,
        };

        let kept_line = parse_line("Kept").unwrap();
        let removed_line = parse_line("Removed").unwrap();

        let choice1 = FullChoiceBuilder::from_line(removed_line.clone())
            .with_conditions(&[unfulfilled_condition.clone()])
            .build();
        let choice2 = FullChoiceBuilder::from_line(kept_line.clone())
            .with_conditions(&[fulfilled_condition.clone()])
            .build();
        let choice3 = FullChoiceBuilder::from_line(removed_line.clone())
            .with_conditions(&[unfulfilled_condition.clone()])
            .build();

        let choices = vec![
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice1,
            },
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice2,
            },
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice3,
            },
        ];

        let displayed_choices =
            prepare_choices_for_user(&choices, &current_address, &knots).unwrap();

        assert_eq!(displayed_choices.len(), 1);
        assert_eq!(&displayed_choices[0].text, "Kept");
    }

    #[test]
    fn preparing_choices_filters_choices_which_have_been_visited_for_non_sticky_lines() {
        let empty_hash_map = HashMap::new();
        let empty_address = Address {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        let kept_line = parse_line("Kept").unwrap();
        let removed_line = parse_line("Removed").unwrap();

        let choice1 = FullChoiceBuilder::from_line(kept_line.clone()).build();
        let choice2 = FullChoiceBuilder::from_line(removed_line.clone()).build();
        let choice3 = FullChoiceBuilder::from_line(kept_line.clone()).build();

        let choices = vec![
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice1,
            },
            ChoiceExtra {
                num_visited: 1,
                choice_data: choice2,
            },
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice3,
            },
        ];

        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Kept");
        assert_eq!(&displayed_choices[1].text, "Kept");
    }

    #[test]
    fn preparing_choices_does_not_filter_visited_sticky_lines() {
        let empty_hash_map = HashMap::new();
        let empty_address = Address {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        let kept_line = parse_line("Kept").unwrap();
        let removed_line = parse_line("Removed").unwrap();

        let choice1 = FullChoiceBuilder::from_line(kept_line.clone()).build();
        let choice2 = FullChoiceBuilder::from_line(removed_line.clone()).build();
        let choice3 = FullChoiceBuilder::from_line(kept_line.clone())
            .is_sticky()
            .build();

        let choices = vec![
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice1,
            },
            ChoiceExtra {
                num_visited: 1,
                choice_data: choice2,
            },
            ChoiceExtra {
                num_visited: 1,
                choice_data: choice3,
            },
        ];

        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Kept");
        assert_eq!(&displayed_choices[1].text, "Kept");
    }

    #[test]
    fn preparing_choices_filters_fallback_choices() {
        let empty_hash_map = HashMap::new();
        let empty_address = Address {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        let kept_line = parse_line("Kept").unwrap();
        let removed_line = parse_line("Removed").unwrap();

        let choice1 = FullChoiceBuilder::from_line(kept_line.clone()).build();
        let choice2 = FullChoiceBuilder::from_line(removed_line.clone())
            .is_fallback()
            .build();
        let choice3 = FullChoiceBuilder::from_line(kept_line.clone())
            .is_sticky()
            .build();

        let choices = vec![
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice1,
            },
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice2,
            },
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice3,
            },
        ];

        let displayed_choices =
            prepare_choices_for_user(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(displayed_choices.len(), 2);
        assert_eq!(&displayed_choices[0].text, "Kept");
        assert_eq!(&displayed_choices[1].text, "Kept");
    }

    #[test]
    fn invalid_choice_error_is_filled_in_with_all_presented_choices() {

        let line1 = parse_line("Choice 1").unwrap();
        let line2 = parse_line("Choice 2").unwrap();

        let choice1 = FullChoiceBuilder::from_line(line1.clone()).build();
        let choice2 = FullChoiceBuilder::from_line(line2.clone()).build();

        let internal_choices = vec![
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice1,
            },
            ChoiceExtra {
                num_visited: 1,
                choice_data: choice2,
            },
        ];

        let made_choice = Choice {
            text: "Made this choice".to_string(),
            tags: Vec::new(),
            index: 5,
        };

        let error = InklingError::InvalidChoice {
            index: 2,
            choice: None,
            presented_choices: Vec::new(),
            internal_choices: internal_choices.clone(),
        };

        let knots = HashMap::new();
        let empty_address = Address {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        let filled_error =
            fill_in_invalid_error(error.clone(), &made_choice, &empty_address, &knots);

        match (filled_error, error) {
            (
                InklingError::InvalidChoice {
                    index: filled_index,
                    choice: filled_choice,
                    presented_choices: filled_presented_choices,
                    internal_choices: filled_internal_choices,
                },
                InklingError::InvalidChoice {
                    index,
                    internal_choices,
                    ..
                },
            ) => {
                assert_eq!(filled_index, index);
                assert_eq!(filled_internal_choices, internal_choices);

                assert_eq!(filled_choice, Some(made_choice));
                assert_eq!(filled_presented_choices.len(), 2);

                let (shown1, choice1) = &filled_presented_choices[0];
                assert!(shown1);
                assert_eq!(choice1.text, "Choice 1");

                let (shown2, choice2) = &filled_presented_choices[1];
                assert!(!shown2);
                assert_eq!(choice2.text, "Choice 2");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn fallback_choices_are_filtered_as_usual_choices() {
        let empty_hash_map = HashMap::new();
        let empty_address = Address {
            knot: "".to_string(),
            stitch: "".to_string(),
        };

        let kept_line = parse_line("Kept").unwrap();
        let removed_line = parse_line("Removed").unwrap();

        let choice1 = FullChoiceBuilder::from_line(kept_line.clone())
            .is_fallback()
            .build();
        let choice2 = FullChoiceBuilder::from_line(removed_line.clone())
            .is_fallback()
            .build();
        let choice3 = FullChoiceBuilder::from_line(kept_line.clone())
            .is_sticky()
            .is_fallback()
            .build();

        let choices = vec![
            ChoiceExtra {
                num_visited: 0,
                choice_data: choice1,
            },
            ChoiceExtra {
                num_visited: 1,
                choice_data: choice2,
            },
            ChoiceExtra {
                num_visited: 1,
                choice_data: choice3,
            },
        ];

        let fallback_choices =
            get_fallback_choices(&choices, &empty_address, &empty_hash_map).unwrap();

        assert_eq!(fallback_choices.len(), 2);
        assert_eq!(&fallback_choices[0].text, "Kept");
        assert_eq!(&fallback_choices[1].text, "Kept");
    }
}
