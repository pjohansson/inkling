use crate::error::{LineError, ParseError};

use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq)]
/// Condition to show choice (or maybe part of line, in the future)
pub enum Condition {
    /// Use a knot (or maybe other string-like variable) to check whether its value 
    /// compares to the set condition.
    NumVisits {
        name: String,
        rhs_value: i32,
        ordering: Ordering,
        not: bool,              // negation of the condition, ie. !(condition)
    },
}

pub fn parse_choice_conditions(line: &mut String) -> Result<Vec<Condition>, ParseError> {
    let mut conditions = Vec::new();
    let full_line = line.clone();

    while let Some(inside) = get_string_inside_brackets(line)? {
        let condition = parse_condition(&inside).map_err(|condition| LineError::BadCondition {
            condition,
            full_line: full_line.clone(),
        })?;

        conditions.push(condition);
    }

    Ok(conditions)
}

fn parse_condition(line: &str) -> Result<Condition, String> {
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
            let rhs_value = tail.parse::<i32>().map_err(|_| line.to_string())? + adjustment;

            Ok(Condition::NumVisits {
                name,
                rhs_value,
                ordering,
                not,
            })
        }
        None => {
            let (name, not) = get_name_and_if_not_condition(line).map_err(|_| line.to_string())?;

            Ok(Condition::NumVisits {
                name,
                rhs_value: 0,
                ordering: Ordering::Greater,
                not,
            })
        }
    }
}

fn get_name_and_if_not_condition(line: &str) -> Result<(String, bool), String> {
    let words = line.trim().split_whitespace().collect::<Vec<_>>();

    if words.len() == 1 {
        Ok((words[0].to_string(), false))
    } else if words.len() == 2 && words[0].to_lowercase() == "not" {
        Ok((words[1].to_string(), true))
    } else {
        Err(line.to_string())
    }
}

fn get_string_inside_brackets(line: &mut String) -> Result<Option<String>, ParseError> {
    match (line.find('{'), line.find('}')) {
        (None, None) => Ok(None),
        (Some(i), Some(j)) if i < j => {
            let mut inside: String = line.drain(..j + 1).take(j).collect();
            inside.drain(..i + 1);

            Ok(Some(inside))
        }
        _ => Err(LineError::UnmatchedBrackets {
            line: line.to_string(),
        }
        .into()),
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
}
