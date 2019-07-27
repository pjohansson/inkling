//! Parse `Condition` objects.

use std::cmp::Ordering;

use crate::{
    line::{
        parse::{split_line_into_variants, LinePart},
        Condition, LineErrorKind, LineParsingError,
    },
};

/// Parse conditions for a choice and trim them from the line.
///
/// Choices can lead with multiple conditions. Every condition is contained inside
/// `{}` bracket pairs and may be whitespace separated. This function reads all conditions
/// until no bracket pairs are left in the leading part of the line.
pub fn parse_choice_conditions(line: &mut String) -> Result<Vec<Condition>, LineParsingError> {
    let full_line = line.clone();

    split_choice_conditions_off_string(line)?
        .into_iter()
        .map(|content| {
            parse_condition(&content)
                .map_err(|err| LineParsingError::from_kind(&full_line, err.kind))
        })
        .collect()
}

fn split_choice_conditions_off_string(
    content: &mut String,
) -> Result<Vec<String>, LineParsingError> {
    let (head, backslash_adjustor) = content
        .find("\\{")
        .and_then(|i| content.get(..i))
        .map(|s| (s, 1))
        .unwrap_or((content.as_str(), 0));

    let parts = split_line_into_variants(head)?;

    let iter = parts.into_iter().take_while(|part| match part {
        LinePart::Embraced(..) => true,
        LinePart::Text(text) => text.chars().all(|c| c.is_whitespace()),
    });

    let num_chars = iter
        .clone()
        .map(|part| match part {
            LinePart::Embraced(text) => text.len() + 2,
            LinePart::Text(text) => text.len(),
        })
        .sum::<usize>();

    let conditions = iter
        .filter_map(|part| match part {
            LinePart::Embraced(text) => Some(text.to_string()),
            _ => None,
        })
        .collect();

    content.drain(..num_chars + backslash_adjustor);

    Ok(conditions)
}

/// Parse a condition from a line.
fn parse_condition(line: &str) -> Result<Condition, LineParsingError> {
    let ordering_search = line
        .find("==")
        .map(|i| (i, Ordering::Equal, 0, 2))
        .or(line.find("<=").map(|i| (i, Ordering::Less, 1, 2)))
        .or(line.find(">=").map(|i| (i, Ordering::Greater, -1, 2)))
        .or(line.find("<").map(|i| (i, Ordering::Less, 0, 1)))
        .or(line.find(">").map(|i| (i, Ordering::Greater, 0, 1)));

    match ordering_search {
        Some((index, ordering, adjustment, symbol_length)) => {
            let head = line.get(..index).unwrap().trim();
            let tail = line.get(index + symbol_length..).unwrap().trim();

            let (name, not) = get_name_and_if_not_condition(head)?;
            let rhs_value = tail.parse::<i32>().map_err(|_| {
                LineParsingError::from_kind(
                    line,
                    LineErrorKind::ExpectedNumber {
                        value: tail.to_string(),
                    },
                )
            })? + adjustment;

            Ok(Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            })
        }
        None => {
            let (name, not) = get_name_and_if_not_condition(line)?;

            Ok(Condition::NumVisits {
                name,
                rhs_value: 0,
                ordering: Ordering::Greater,
                not,
            })
        }
    }
}

/// Parse the condition `name` and whether the condition is negated.
///
/// Conditions are of the form {(not) name (op value)} and this function treats
/// the line that is left after trimming the (op value) part from it. Thus, we want
/// to get the name and whether a `not` statement preceedes it.
fn get_name_and_if_not_condition(line: &str) -> Result<(String, bool), LineParsingError> {
    let words = line.trim().split_whitespace().collect::<Vec<_>>();

    if words.len() == 1 {
        Ok((words[0].to_string(), false))
    } else if words.len() == 2 && words[0].to_lowercase() == "not" {
        Ok((words[1].to_string(), true))
    } else {
        Err(LineParsingError::from_kind(
            line,
            LineErrorKind::ExpectedLogic {
                line: line.to_string(),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_condition_without_brackets_return_none_and_leaves_string_unchanged() {
        let mut line = "Hello, World!".to_string();
        assert!(parse_choice_conditions(&mut line).unwrap().is_empty());
    }

    #[test]
    fn parsing_bad_conditions_give_error() {
        assert!(parse_condition("not too many names").is_err());
        assert!(parse_condition("not too many > 3").is_err());
        assert!(parse_condition("no_value >").is_err());
        assert!(parse_condition("too_many_values > 3 2").is_err());
        assert!(parse_condition("bad_value > s").is_err());
        assert!(parse_condition("").is_err());
    }

    #[test]
    fn parsing_condition_with_just_name_gives_larger_than_zero_visits_condition() {
        let mut line = "{knot_name} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        match &conditions[0] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "knot_name");
                assert_eq!(*rhs_value, 0);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, false);
            }
        }
    }

    #[test]
    fn choice_conditions_are_only_parsed_for_braces_before_text_content() {
        let mut line = "Hello, World! {knot_name}".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        assert_eq!(conditions.len(), 0);
    }

    #[test]
    fn braces_starting_with_backslash_are_not_conditions_when_parsing_choices() {
        let mut line = "\\{knot_name} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        assert_eq!(&line, "{knot_name} Hello, World!");
        assert_eq!(conditions.len(), 0);
    }

    #[test]
    fn several_conditions_can_be_parsed() {
        let mut line = "{knot_name} {other_knot} {not third_knot} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        assert_eq!(conditions.len(), 3);

        match &conditions[2] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "third_knot");
                assert_eq!(*rhs_value, 0);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, true);
            }
        }
    }

    #[test]
    fn parsing_condition_with_not_sets_reverse_condition() {
        let mut line = "{not knot_name} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        match &conditions[0] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "knot_name");
                assert_eq!(*rhs_value, 0);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, true);
            }
        }
    }

    #[test]
    fn parsing_single_larger_than_condition() {
        let mut line = "{knot_name > 2} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        match &conditions[0] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "knot_name");
                assert_eq!(*rhs_value, 2);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, false);
            }
        }
    }

    #[test]
    fn parsing_single_not_larger_than_condition() {
        let mut line = "{not knot_name > 2} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        match &conditions[0] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "knot_name");
                assert_eq!(*rhs_value, 2);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, true);
            }
        }
    }

    #[test]
    fn parsing_single_less_than_condition() {
        let mut line = "{knot_name < 2} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        match &conditions[0] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "knot_name");
                assert_eq!(*rhs_value, 2);
                assert_eq!(*ordering, Ordering::Less);
                assert_eq!(*not, false);
            }
        }
    }

    #[test]
    fn parsing_single_equal_than_condition() {
        let mut line = "{knot_name == 2} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        match &conditions[0] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "knot_name");
                assert_eq!(*rhs_value, 2);
                assert_eq!(*ordering, Ordering::Equal);
                assert_eq!(*not, false);
            }
        }
    }

    #[test]
    fn parsing_single_equal_to_or_greater_than_condition() {
        let mut line = "{knot_name >= 2} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        match &conditions[0] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "knot_name");
                assert_eq!(*rhs_value, 1);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, false);
            }
        }
    }

    #[test]
    fn parsing_single_equal_to_or_less_than_condition() {
        let mut line = "{knot_name <= 2} Hello, World!".to_string();
        let conditions = parse_choice_conditions(&mut line).unwrap();

        match &conditions[0] {
            Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(name, "knot_name");
                assert_eq!(*rhs_value, 3);
                assert_eq!(*ordering, Ordering::Less);
                assert_eq!(*not, false);
            }
        }
    }

    #[test]
    fn splitting_choice_conditions_removes_initial_braces_from_line() {
        let mut line = "{condition} {condition} Hello, World!".to_string();
        split_choice_conditions_off_string(&mut line).unwrap();

        assert_eq!(&line, " Hello, World!");

        let mut line = " Hello, World! ".to_string();
        split_choice_conditions_off_string(&mut line).unwrap();

        assert_eq!(&line, " Hello, World! ");
    }

    #[test]
    fn splitting_choice_conditions_with_multibyte_characters_splits_string_correctly() {
        let mut line = "{김택용} Hello, World!".to_string();
        split_choice_conditions_off_string(&mut line).unwrap();

        assert_eq!(&line, " Hello, World!");

        let mut line = " Hello, World! ".to_string();
        split_choice_conditions_off_string(&mut line).unwrap();

        assert_eq!(&line, " Hello, World! ");
    }

    #[test]
    fn splitting_choice_conditions_returns_braced_conditions_as_strings() {
        let mut line = "{condition_one} {condition_two} Hello, World!".to_string();
        let conditions = split_choice_conditions_off_string(&mut line).unwrap();

        assert_eq!(conditions.len(), 2);
        assert_eq!(&conditions[0], "condition_one");
        assert_eq!(&conditions[1], "condition_two");
    }
}
