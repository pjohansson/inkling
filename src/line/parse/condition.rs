//! Parse `StoryCondition` objects.

use std::cmp::Ordering;

use crate::{
    error::{BadCondition, BadConditionKind, LineErrorKind, LineParsingError},
    line::{
        parse::{
            split_line_at_separator_braces, split_line_at_separator_parenthesis,
            split_line_into_groups_braces, split_line_into_groups_parenthesis, LinePart,
        },
        Condition, ConditionBuilder, ConditionItem, ConditionKind, StoryCondition,
    },
    story::Address,
};

/// Parse conditions from a line of content.
///
/// Returns the conditions along with the string to process if the condition is fulfilled
/// and, if present, the string to process if the condition is false.
///
/// Conditional content is marked by being off the format {condition: if true | else}.
/// This is then how we parse conditions.
///
/// # Notes
/// *   Choices have a separate way of marking conditions for them to be presented. See
///     `parse_choice_conditions` for such parsing. Of course, line content within a choice
///     may be marked up using conditions that will use this format.
pub fn parse_line_condition(
    line: &str,
) -> Result<(Condition, &str, Option<&str>), LineParsingError> {
    let (condition_content, true_content, false_content) = split_line_condition_content(line)?;

    let false_line = if !false_content.trim().is_empty() {
        Some(false_content)
    } else {
        None
    };

    // Ok((
    //     parse_conditions(condition_content)?,
    //     true_content,
    //     false_line,
    // ))

    unimplemented!();
}

/// Parse conditions for a choice and trim them from the line.
///
/// Choices can lead with multiple conditions. Every condition is contained inside
/// `{}` bracket pairs and may be whitespace separated. This function reads all conditions
/// until no bracket pairs are left in the leading part of the line.
pub fn parse_choice_condition(line: &mut String) -> Result<Option<Condition>, LineParsingError> {
    let full_line = line.clone();

    let conditions = split_choice_conditions_off_string(line)?
        .into_iter()
        .map(|content| {
            parse_story_condition(&content)
                .map_err(|err| LineParsingError::from_kind(&full_line, err.kind))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let condition = conditions.split_first().map(|(first, rest)| {
        let mut builder = ConditionBuilder::from_kind(&first.into(), false);

        for kind in rest {
            builder.and(&kind.into(), false);
        }

        builder.build()
    });

    Ok(condition)
}

/// Split conditions from the string and return them separately.
///
/// In this case, splitting from the string means that all characters up until the regular
/// line begins are removed from the input string as a side effect. The string up until
/// that point is split into string expressions of the conditions contained within them.
///
/// # Notes
/// *   ConditionKinds are marked by being contained within curly '{}' braces. This is unique
///     to parsing conditions for choices. Other conditional lines require separate markup.
/// *   As soon as text which is not enclosed by braces appear the condition parsing
///     ends.
/// *   A backslash '\\' can be used in front of a curly brace to denote that it's not
///     a condition.
/// *   The condition strings are returned without the enclosing braces.
fn split_choice_conditions_off_string(
    content: &mut String,
) -> Result<Vec<String>, LineParsingError> {
    let (head, backslash_adjustor) = content
        .find("\\{")
        .and_then(|i| content.get(..i))
        .map(|s| (s, 1))
        .unwrap_or((content.as_str(), 0));

    let parts = split_line_into_groups_braces(head)?;

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

#[derive(Clone, Copy, Debug, PartialEq)]
enum Link {
    And,
    Or,
    Blank,
}

/// Parse all conditions present in a line.
fn parse_condition(content: &str) -> Result<Condition, BadCondition> {
    let mut buffer = content.to_string();

    let mut items: Vec<(Link, ConditionItem)> = Vec::new();

    while !buffer.trim().is_empty() {
        let mut condition_string = read_next_condition_string(&mut buffer)?;

        let link = split_off_condition_link(&mut condition_string);
        let negate = split_off_negation(&mut condition_string);

        let condition_kind = parse_condition_kind(&condition_string)?;
        let item = ConditionItem {
            kind: condition_kind,
            negate,
        };

        items.push((link, item));
    }

    items
        .split_first()
        .map(|((_, first), tail)| {
            let mut builder = ConditionBuilder::from_kind(&first.kind, first.negate);

            for (link, item) in tail {
                match link {
                    Link::And => builder.and(&item.kind, item.negate),
                    Link::Or => builder.or(&item.kind, item.negate),
                    Link::Blank => unimplemented!(),
                }
            }

            builder.build()
        })
        .ok_or(BadCondition::from_kind(
            content,
            BadConditionKind::NoCondition,
        ))
}

fn parse_condition_kind(content: &str) -> Result<ConditionKind, BadCondition> {
    if &content.trim().to_lowercase() == "true" {
        Ok(ConditionKind::True)
    } else if &content.trim().to_lowercase() == "false" {
        Ok(ConditionKind::False)
    } else if content.trim().starts_with('(') && content.trim().ends_with(')') {
        let i = content.find('(').unwrap();
        let j = content.find(')').unwrap();

        let inner_block = content.get(i + 1..j).unwrap();

        let condition = parse_condition(inner_block)?;

        Ok(ConditionKind::Nested(Box::new(condition)))
    } else {
        let condition = parse_story_condition(content)
            .map_err(|_| BadCondition::from_kind(content, BadConditionKind::CouldNotParse))?;

        Ok(ConditionKind::Single(condition))
    }
}

fn split_off_condition_link(content: &mut String) -> Link {
    let buffer = content.to_lowercase();

    let (link, len) = if buffer.starts_with("and") {
        (Link::And, 3)
    } else if buffer.starts_with("&&") {
        (Link::And, 2)
    } else if buffer.starts_with("or") || buffer.starts_with("||") {
        (Link::Or, 2)
    } else {
        (Link::Blank, 0)
    };

    content.drain(..len);

    link
}

fn split_off_negation(content: &mut String) -> bool {
    if content.to_lowercase().trim_start().starts_with("not ") {
        let index = content.to_lowercase().find("not").unwrap();
        content.drain(..index + 3);

        true
    } else {
        false
    }
}

fn read_next_condition_string(buffer: &mut String) -> Result<String, BadCondition> {
    let (head, tail) = get_without_starting_match(&buffer);

    let index = get_closest_split_index(tail).map_err(|_| {
        BadCondition::from_kind(buffer.as_str(), BadConditionKind::UnmatchedParenthesis)
    })?;

    Ok(buffer.drain(..index + head.len()).collect())
}

fn get_without_starting_match(content: &str) -> (&str, &str) {
    let buffer = content.to_lowercase();

    let index = if buffer.starts_with("and") {
        3
    } else if buffer.starts_with("or") || buffer.starts_with("||") || buffer.starts_with("&&") {
        2
    } else {
        0
    };

    content.split_at(index)
}

fn get_closest_split_index(content: &str) -> Result<usize, LineParsingError> {
    let buffer = content.to_lowercase();

    get_split_index(&buffer, "and")
        .and_then(|current_min| get_split_index(&buffer, "&&").map(|next| current_min.min(next)))
        .and_then(|current_min| get_split_index(&buffer, "or").map(|next| current_min.min(next)))
        .and_then(|current_min| get_split_index(&buffer, "||").map(|next| current_min.min(next)))
}

fn get_split_index(content: &str, separator: &str) -> Result<usize, LineParsingError> {
    split_line_at_separator_parenthesis(content, separator, Some(1))
        .map(|parts| parts[0].as_bytes().len())
}

/// Parse a condition from a line.
fn parse_story_condition(line: &str) -> Result<StoryCondition, LineParsingError> {
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

            Ok(StoryCondition::NumVisits {
                address: Address::Raw(name.to_string()),
                rhs_value,
                ordering,
                not,
            })
        }
        None => {
            let (name, not) = get_name_and_if_not_condition(line)?;

            Ok(StoryCondition::NumVisits {
                address: Address::Raw(name.to_string()),
                rhs_value: 0,
                ordering: Ordering::Greater,
                not,
            })
        }
    }
}

/// Split a line into conditional, true and false parts.
fn split_line_condition_content(content: &str) -> Result<(&str, &str, &str), LineParsingError> {
    let parts = split_line_at_separator_braces(content, ":", Some(1))?;

    let (head, tail) = match parts.len() {
        1 => Err(LineParsingError::from_kind(
            content,
            BadCondition::from_kind(content, BadConditionKind::NoCondition).into(),
        )),
        2 => Ok((parts[0], parts[1])),
        _ => unreachable!(),
    }?;

    let variational_parts = split_line_at_separator_braces(tail, "|", Some(2))?;

    match variational_parts.len() {
        1 => Ok((head, variational_parts[0], "")),
        2 => Ok((head, variational_parts[0], variational_parts[1])),
        _ => Err(LineParsingError::from_kind(
            content,
            BadCondition::from_kind(content, BadConditionKind::MultipleElseStatements).into(),
        )),
    }
}

/// Parse the condition `name` and whether the condition is negated.
///
/// ConditionKinds are of the form {(not) name (op value)} and this function treats
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
        assert!(parse_choice_condition(&mut line).unwrap().is_none());
    }

    #[test]
    fn parsing_bad_conditions_give_error() {
        assert!(parse_story_condition("not too many names").is_err());
        assert!(parse_story_condition("not too many > 3").is_err());
        assert!(parse_story_condition("no_value >").is_err());
        assert!(parse_story_condition("too_many_values > 3 2").is_err());
        assert!(parse_story_condition("bad_value > s").is_err());
        assert!(parse_story_condition("").is_err());
    }

    #[test]
    fn parsing_condition_with_just_name_gives_larger_than_zero_visits_condition() {
        let mut line = "{knot_name} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        match &condition.story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("knot_name".to_string()));
                assert_eq!(*rhs_value, 0);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, false);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn choice_conditions_are_only_parsed_for_braces_before_text_content() {
        let mut line = "Hello, World! {knot_name}".to_string();
        let conditions = parse_choice_condition(&mut line).unwrap();

        assert!(conditions.is_none());
    }

    #[test]
    fn braces_starting_with_backslash_are_not_conditions_when_parsing_choices() {
        let mut line = "\\{knot_name} Hello, World!".to_string();
        let conditions = parse_choice_condition(&mut line).unwrap();

        assert_eq!(&line, "{knot_name} Hello, World!");
        assert!(conditions.is_none());
    }

    #[test]
    fn several_choice_conditions_can_be_parsed_and_will_be_and_variants() {
        let mut line = "{knot_name} {other_knot} {not third_knot} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        assert_eq!(condition.items.len(), 2);

        match &condition.items[1].story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("third_knot".to_string()));
                assert_eq!(*rhs_value, 0);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, true);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parsing_condition_with_not_sets_reverse_condition() {
        let mut line = "{not knot_name} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        match &condition.story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("knot_name".to_string()));
                assert_eq!(*rhs_value, 0);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, true);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parsing_single_larger_than_condition() {
        let mut line = "{knot_name > 2} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        match &condition.story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("knot_name".to_string()));
                assert_eq!(*rhs_value, 2);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, false);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parsing_single_not_larger_than_condition() {
        let mut line = "{not knot_name > 2} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        match &condition.story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("knot_name".to_string()));
                assert_eq!(*rhs_value, 2);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, true);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parsing_single_less_than_condition() {
        let mut line = "{knot_name < 2} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        match &condition.story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("knot_name".to_string()));
                assert_eq!(*rhs_value, 2);
                assert_eq!(*ordering, Ordering::Less);
                assert_eq!(*not, false);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parsing_single_equal_than_condition() {
        let mut line = "{knot_name == 2} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        match &condition.story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("knot_name".to_string()));
                assert_eq!(*rhs_value, 2);
                assert_eq!(*ordering, Ordering::Equal);
                assert_eq!(*not, false);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parsing_single_equal_to_or_greater_than_condition() {
        let mut line = "{knot_name >= 2} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        match &condition.story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("knot_name".to_string()));
                assert_eq!(*rhs_value, 1);
                assert_eq!(*ordering, Ordering::Greater);
                assert_eq!(*not, false);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parsing_single_equal_to_or_less_than_condition() {
        let mut line = "{knot_name <= 2} Hello, World!".to_string();
        let condition = parse_choice_condition(&mut line).unwrap().unwrap();

        match &condition.story_condition() {
            StoryCondition::NumVisits {
                address,
                rhs_value,
                ordering,
                not,
            } => {
                assert_eq!(address, &Address::Raw("knot_name".to_string()));
                assert_eq!(*rhs_value, 3);
                assert_eq!(*ordering, Ordering::Less);
                assert_eq!(*not, false);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parsing_condition_kind_returns_true_or_false_for_those_strings() {
        assert_eq!(
            parse_condition_kind("  true  ").unwrap(),
            ConditionKind::True
        );
        assert_eq!(
            parse_condition_kind("  TRUE  ").unwrap(),
            ConditionKind::True
        );
        assert_eq!(
            parse_condition_kind("  false  ").unwrap(),
            ConditionKind::False
        );
        assert_eq!(
            parse_condition_kind("  FALSE  ").unwrap(),
            ConditionKind::False
        );
    }

    #[test]
    fn parsing_simple_condition_returns_a_story_condition() {
        match parse_condition_kind("root").unwrap() {
            ConditionKind::Single(..) => (),
            other => panic!("expected `ConditionKind::Single` but got {:?}", other),
        }

        match parse_condition_kind("root > 1").unwrap() {
            ConditionKind::Single(..) => (),
            other => panic!("expected `ConditionKind::Single` but got {:?}", other),
        }

        match parse_condition_kind("  root<=1   ").unwrap() {
            ConditionKind::Single(..) => (),
            other => panic!("expected `ConditionKind::Single` but got {:?}", other),
        }
    }

    #[test]
    fn parsing_condition_with_surrounding_parenthesis_returns_nested_condition() {
        match parse_condition_kind("(root)").unwrap() {
            ConditionKind::Nested(..) => (),
            other => panic!("expected `ConditionKind::Nested` but got {:?}", other),
        }

        match parse_condition_kind("(root or root)").unwrap() {
            ConditionKind::Nested(..) => (),
            other => panic!("expected `ConditionKind::Nested` but got {:?}", other),
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

    #[test]
    fn condition_strings_with_just_condition_and_content_splits_at_colon() {
        assert_eq!(
            split_line_condition_content("condition :").unwrap(),
            ("condition ", "", "")
        );

        assert_eq!(
            split_line_condition_content("condition : content").unwrap(),
            ("condition ", " content", "")
        );
    }

    #[test]
    fn condition_string_with_vertical_line_after_colon_separates_that_into_tail() {
        assert_eq!(
            split_line_condition_content("condition : true |").unwrap(),
            ("condition ", " true ", "")
        );

        assert_eq!(
            split_line_condition_content("condition : true | false").unwrap(),
            ("condition ", " true ", " false")
        );
    }

    #[test]
    fn vertical_line_before_colon_in_condition_line_still_splits_colon_first() {
        assert_eq!(
            split_line_condition_content("cond | ition : true | false").unwrap(),
            ("cond | ition ", " true ", " false")
        );
    }

    #[test]
    fn no_colon_separator_in_condition_content_yields_error() {
        assert!(split_line_condition_content("content without condition").is_err());
        assert!(split_line_condition_content("content | condition").is_err());
    }

    #[test]
    fn multiple_vertical_lines_in_condition_content_yields_error() {
        assert!(split_line_condition_content("condition : true | false | again").is_err());
    }

    #[test]
    fn splitting_off_condition_link_splits_off_exactly_those_chars() {
        let mut buffer = "and or || && rest".to_string();

        assert_eq!(split_off_condition_link(&mut buffer), Link::And);
        assert_eq!(&buffer, " or || && rest");

        buffer = buffer.trim_start().to_string();
        assert_eq!(split_off_condition_link(&mut buffer), Link::Or);
        assert_eq!(&buffer, " || && rest");

        buffer = buffer.trim_start().to_string();
        assert_eq!(split_off_condition_link(&mut buffer), Link::Or);
        assert_eq!(&buffer, " && rest");

        buffer = buffer.trim_start().to_string();
        assert_eq!(split_off_condition_link(&mut buffer), Link::And);
        assert_eq!(&buffer, " rest");
    }

    #[test]
    fn splitting_off_negation_removes_beginning_not() {
        let mut buffer = "  not rest".to_string();

        assert!(split_off_negation(&mut buffer));
        assert_eq!(&buffer, " rest");

        assert!(!split_off_negation(&mut buffer));
        assert_eq!(&buffer, " rest");
    }

    #[test]
    fn closest_split_index_works_for_all_variants() {
        assert_eq!(get_closest_split_index("1 and 2 or 3").unwrap(), 2);
        assert_eq!(get_closest_split_index("1 or 2 and 3").unwrap(), 2);
        assert_eq!(get_closest_split_index("1 || 2 or 3").unwrap(), 2);
        assert_eq!(get_closest_split_index("1 && 2 || 3").unwrap(), 2);
    }

    #[test]
    fn next_condition_starts_when_and_or_or_is_encountered_after_address() {
        let mut buffer = "knot or knot and knot".to_string();

        assert_eq!(&read_next_condition_string(&mut buffer).unwrap(), "knot ");
        assert_eq!(&buffer, "or knot and knot");

        assert_eq!(
            &read_next_condition_string(&mut buffer).unwrap(),
            "or knot "
        );
        assert_eq!(&buffer, "and knot");

        assert_eq!(
            &read_next_condition_string(&mut buffer).unwrap(),
            "and knot"
        );
        assert_eq!(&buffer, "");
    }

    #[test]
    fn separators_inside_parenthesis_are_ignored() {
        let mut buffer = "knot or (knot and knot) and knot".to_string();

        assert_eq!(&read_next_condition_string(&mut buffer).unwrap(), "knot ");
        assert_eq!(
            &read_next_condition_string(&mut buffer).unwrap(),
            "or (knot and knot) "
        );
        assert_eq!(
            &read_next_condition_string(&mut buffer).unwrap(),
            "and knot"
        );
    }

    #[test]
    fn and_may_be_ampersands_and_or_may_be_vertical_lines() {
        let mut buffer = "knot || knot && knot".to_string();

        assert_eq!(&read_next_condition_string(&mut buffer).unwrap(), "knot ");
        assert_eq!(
            &read_next_condition_string(&mut buffer).unwrap(),
            "|| knot "
        );
        assert_eq!(&read_next_condition_string(&mut buffer).unwrap(), "&& knot");
    }
}
