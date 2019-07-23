use crate::line::ParsedLine;

use super::node::{DialogueNode, NodeItem, NodeType};

/// Used to parse the input lines from beginning to end, returning the constructed `DialogueNode`.
pub fn parse_full_node(lines: &[ParsedLine]) -> DialogueNode {
    let mut items = Vec::new();

    let mut index = 0;

    while index < lines.len() {
        let line = &lines[index];

        match line {
            ParsedLine::Line(line) => {
                let item = NodeItem::Line(line.clone());
                items.push(item);
            }
            ParsedLine::Choice { level, .. } => {
                let (item, gather) = parse_choice_set_with_gather(&mut index, *level, lines);
                items.push(item);

                if let Some(line) = gather {
                    items.push(line);

                    // `parse_choice_set_with_gather` advances the index to the next line
                    // after this group if a gather was found, but this loop also does that
                    // at every iteration. Retract the index once to compensate.
                    index -= 1;
                }
            }
            ParsedLine::Gather { line, .. } => {
                let item = NodeItem::Line(line.clone());
                items.push(item);
            }
        };

        index += 1;
    }

    DialogueNode::with_items(items)
}

/// Used to parse the input lines from beginning to end, returning the constructed `DialogueNode`.
pub fn new_parse_full_node(lines: &[ParsedLine]) -> RootNode {
    let mut builder = RootNodeBuilder::new();

    let mut index = 0;

    while index < lines.len() {
        let line = &lines[index];

        match line {
            ParsedLine::Line(line) => {
                // let item = NodeItem::Line(line.clone());
                // items.push(item);
                let line = LineBuilder::new()
                    .add_item(LineContainer::Text(line.clone()))
                    .build();
                builder.add_item(NodeContainer::Line(line));
            }
            ParsedLine::Choice { level, .. } => {
                let (item, gather) = new_parse_choice_set_with_gather(&mut index, *level, lines);
                builder.add_item(NodeContainer::BranchingChoice(item));

                if let Some(line) = gather {
                    let line = LineBuilder::new().add_item(line).build();
                    builder.add_item(NodeContainer::Line(line));

                    // `parse_choice_set_with_gather` advances the index to the next line
                    // after this group if a gather was found, but this loop also does that
                    // at every iteration. Retract the index once to compensate.
                    index -= 1;
                }
            }
            ParsedLine::Gather { line, .. } => {
                let line = LineBuilder::new()
                    .add_item(LineContainer::Text(line.clone()))
                    .build();
                builder.add_item(NodeContainer::Line(line));
            }
        };

        index += 1;
    }

    builder.build()
}

/// After parsing a group of choices, check whether it ended because of a `Gather`.
/// If so, return the `NodeItem::Line` object from that gather so that it can be appended
/// *after* the node, not inside it.
fn parse_choice_set_with_gather(
    index: &mut usize,
    current_level: u32,
    lines: &[ParsedLine],
) -> (NodeItem, Option<NodeItem>) {
    let node = parse_choice_set(index, current_level, lines);
    let mut gather = None;

    if let Some(ParsedLine::Gather { level, line }) = lines.get(*index) {
        if *level == current_level {
            gather.replace(NodeItem::Line(line.clone()));
            *index += 1;
        }
    }

    (node, gather)
}

/// After parsing a group of choices, check whether it ended because of a `Gather`.
/// If so, return the `NodeItem::Line` object from that gather so that it can be appended
/// *after* the node, not inside it.
fn new_parse_choice_set_with_gather(
    index: &mut usize,
    current_level: u32,
    lines: &[ParsedLine],
) -> (Vec<Branch>, Option<LineContainer>) {
    let node = new_parse_choice_set(index, current_level, lines);
    let mut gather = None;

    if let Some(ParsedLine::Gather { level, line }) = lines.get(*index) {
        if *level == current_level {
            gather.replace(LineContainer::Text(line.clone()));
            *index += 1;
        }
    }

    (node, gather)
}

/// Parse a set of `Choice`s with the same level, grouping them into a single `ChoiceSet`
/// node that is returned.
fn parse_choice_set(index: &mut usize, current_level: u32, lines: &[ParsedLine]) -> NodeItem {
    let mut choices = Vec::new();

    while let Some(choice) = parse_single_choice(index, current_level, lines) {
        choices.push(choice);
    }

    let node = DialogueNode::with_items(choices);

    NodeItem::Node {
        kind: NodeType::ChoiceSet,
        node: Box::new(node),
    }
}

/// Parse a set of `Choice`s with the same level, grouping them into a single `ChoiceSet`
/// node that is returned.
fn new_parse_choice_set(
    index: &mut usize,
    current_level: u32,
    lines: &[ParsedLine],
) -> Vec<Branch> {
    // let mut choices = Vec::new();
    // let mut branching_set = BranchingChoiceBuilder::new();

    // while let Some(branch) = new_parse_single_choice(index, current_level, lines) {
    //     branching_set.add_branch(branch);
    //     // choices.push(choice);
    // }

    (0..)
        .map(|_| new_parse_single_choice(index, current_level, lines))
        .take_while(|result| result.is_some())
        .map(|result| result.unwrap())
        .collect::<Vec<_>>()

    // branching_set.build()
    // let node = DialogueNode::with_items(choices);

    // NodeItem::Node {
    //     kind: NodeType::ChoiceSet,
    //     node: Box::new(node),
    // }
}

/// Parse a single `Choice` node. The node ends either when another `Choice` node with
/// the same level or below is encountered, when a `Gather` with the same level or below
/// is encountered or when all lines are read.
fn parse_single_choice(
    index: &mut usize,
    current_level: u32,
    lines: &[ParsedLine],
) -> Option<NodeItem> {
    if *index >= lines.len() {
        return None;
    }

    let mut items = Vec::new();

    let head = &lines[*index];

    let choice = match head {
        ParsedLine::Choice { level, .. } if *level < current_level => {
            return None;
        }
        ParsedLine::Gather { level, .. } if *level <= current_level => {
            return None;
        }
        ParsedLine::Choice { choice, .. } => choice.clone(),
        _ => panic!(
            "could not correctly parse a `NodeItem` of type `Choice`: \
             expected first line to be a `ParsedLine::Choice` object, but was {:?}",
            &head
        ),
    };

    items.push(NodeItem::Line(choice.line.clone()));

    // This skips to the next index, where the choice's content or a new choice will appear
    *index += 1;

    while *index < lines.len() {
        let line = &lines[*index];

        match line {
            ParsedLine::Line(line) => {
                let item = NodeItem::Line(line.clone());
                items.push(item);
            }
            ParsedLine::Choice { level, .. } if *level == current_level => break,
            ParsedLine::Choice { level, .. } if *level > current_level => {
                let (multi_choice, gather) = parse_choice_set_with_gather(index, *level, lines);

                items.push(multi_choice);

                if let Some(line) = gather {
                    items.push(line);
                }

                // `parse_choice_set` advances the index to the next line after the group,
                // but this loop also does that at every iteration. Retract the index once
                // to compensate.
                *index -= 1;
            }
            ParsedLine::Choice { .. } => {
                break;
            }
            ParsedLine::Gather { level, .. } => {
                if *level <= current_level {
                    break;
                }
            }
        }

        *index += 1;
    }

    let node = DialogueNode::with_items(items);

    Some(NodeItem::Node {
        kind: NodeType::Choice(choice),
        node: Box::new(node),
    })
}

use super::node::Container as NodeContainer;
use super::node::*;
use crate::line::Container as LineContainer;
use crate::line::*;

/// Parse a single `Choice` node. The node ends either when another `Choice` node with
/// the same level or below is encountered, when a `Gather` with the same level or below
/// is encountered or when all lines are read.
fn new_parse_single_choice(
    index: &mut usize,
    current_level: u32,
    lines: &[ParsedLine],
) -> Option<Branch> {
    if *index >= lines.len() {
        return None;
    }

    let head = &lines[*index];

    let choice = match head {
        ParsedLine::Choice { level, .. } if *level < current_level => {
            return None;
        }
        ParsedLine::Gather { level, .. } if *level <= current_level => {
            return None;
        }
        ParsedLine::Choice { choice, .. } => choice.clone(),
        _ => panic!(
            "could not correctly parse a `Branch` item: \
             expected first line to be a `ParsedLine::Choice` object, but was {:?}",
            &head
        ),
    };

    let mut builder = BranchBuilder::with_choice(choice);

    // This skips to the next index, where the choice's content or a new choice will appear
    *index += 1;

    while *index < lines.len() {
        let line = &lines[*index];

        match line {
            ParsedLine::Line(line) => {
                let line = LineBuilder::new()
                    .add_item(LineContainer::Text(line.clone()))
                    .build();

                builder.add_item(NodeContainer::Line(line));
            }
            ParsedLine::Choice { level, .. } if *level == current_level => break,
            ParsedLine::Choice { level, .. } if *level > current_level => {
                let (branching_set, gather) =
                    new_parse_choice_set_with_gather(index, *level, lines);

                builder.add_item(NodeContainer::BranchingChoice(branching_set));

                if let Some(line) = gather {
                    let line = LineBuilder::new().add_item(line).build();
                    builder.add_item(NodeContainer::Line(line));
                }

                // `parse_choice_set` advances the index to the next line after the group,
                // but this loop also does that at every iteration. Retract the index once
                // to compensate.
                *index -= 1;
            }
            ParsedLine::Choice { .. } => {
                break;
            }
            ParsedLine::Gather { level, .. } => {
                if *level <= current_level {
                    break;
                }
            }
        }

        *index += 1;
    }

    Some(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    use crate::line::{
        choice::tests::ChoiceBuilder, line::tests::LineBuilder, ChoiceData, LineData,
    };

    // #[test]
    // fn parsing_choices_at_same_level_returns_when_encountering_other_choice() {
    //     let level = 1;
    //     let choice = get_empty_choice(level);

    //     let lines = vec![choice.clone(), choice.clone(), choice.clone()];

    //     let mut index = 0;

    //     let first = parse_single_choice(&mut index, level, &lines).unwrap();
    //     assert_eq!(index, 1);
    //     assert!(first.is_choice());
    //     assert_eq!(first.node().items.len(), 1);

    //     let second = parse_single_choice(&mut index, level, &lines).unwrap();
    //     assert_eq!(index, 2);
    //     assert!(second.is_choice());
    //     assert_eq!(second.node().items.len(), 1);

    //     let third = parse_single_choice(&mut index, level, &lines).unwrap();
    //     assert_eq!(index, 3);
    //     assert!(third.is_choice());
    //     assert_eq!(third.node().items.len(), 1);

    //     assert!(parse_single_choice(&mut index, level, &lines).is_none());
    // }

    #[test]
    fn parsing_a_branch_adds_the_choice_final_line_as_line_in_items() {
        let level = 1;
        let choice = get_empty_choice(level);

        let lines = vec![choice.clone()];

        let mut index = 0;

        let first = new_parse_single_choice(&mut index, level, &lines).unwrap();
        assert_eq!(first.items.len(), 1);
    }

    #[test]
    fn parsing_branches_with_same_level_returns_when_another_branch_is_encountered() {
        let level = 1;
        let choice = get_empty_choice(level);

        let lines = vec![choice.clone(), choice.clone(), choice.clone()];

        let mut index = 0;

        let first = new_parse_single_choice(&mut index, level, &lines).unwrap();
        assert_eq!(first.items.len(), 1);

        let second = new_parse_single_choice(&mut index, level, &lines).unwrap();
        assert_eq!(second.items.len(), 1);

        let third = new_parse_single_choice(&mut index, level, &lines).unwrap();
        assert_eq!(third.items.len(), 1);

        assert!(new_parse_single_choice(&mut index, level, &lines).is_none());
    }

    // #[test]
    // fn parsing_choice_sets_choice_line_in_items_if_not_empty() {
    //     let line = LineData::from_str("Choice line").unwrap();
    //     let choice = ChoiceBuilder::empty().with_line(line.clone()).build();
    //     let parsed_choice = ParsedLine::Choice { level: 1, choice };

    //     let lines = vec![parsed_choice];

    //     let mut index = 0;
    //     let node_item = parse_single_choice(&mut index, 0, &lines).unwrap();

    //     match node_item {
    //         NodeItem::Node { node, .. } => {
    //             assert_eq!(node.items.len(), 1);
    //         }
    //         _ => panic!(),
    //     }
    // }

    // #[test]
    // fn parsing_choice_sets_choice_line_in_items_if_it_has_a_divert() {
    //     let line = LineBuilder::new("").with_divert("to_knot").build();
    //     let choice = ChoiceBuilder::empty().with_line(line.clone()).build();
    //     let parsed_choice = ParsedLine::Choice { level: 1, choice };

    //     let lines = vec![parsed_choice];

    //     let mut index = 0;
    //     let node_item = parse_single_choice(&mut index, 0, &lines).unwrap();

    //     match node_item {
    //         NodeItem::Node { node, .. } => {
    //             assert_eq!(node.items.len(), 1);
    //         }
    //         _ => panic!(),
    //     }
    // }

    // #[test]
    // fn parsing_choice_returns_choice_data_in_root() {
    //     let text = "\"To Netherfield Park, then\", I exclaimed.";

    //     let line = LineData::from_str(text).unwrap();
    //     let choice = ChoiceBuilder::empty()
    //         .with_displayed(line.clone())
    //         .with_line(line.clone())
    //         .build();

    //     let input = ParsedLine::Choice {
    //         level: 1,
    //         choice: choice.clone(),
    //     };

    //     let lines = vec![input];

    //     let mut index = 0;

    //     match parse_single_choice(&mut index, 1, &lines).unwrap() {
    //         NodeItem::Node {
    //             kind: NodeType::Choice(parsed),
    //             ..
    //         } => {
    //             assert_eq!(parsed, choice);
    //         }
    //         _ => panic!("result not a `NodeItem::Node { kind: NodeType::Choice, .. }"),
    //     }
    // }

    #[test]
    fn parsing_a_branch_sets_the_choice_data_in_the_branch_item() {
        let text = "\"To Netherfield Park, then\", I exclaimed.";

        let line = LineData::from_str(text).unwrap();
        let choice = ChoiceBuilder::empty()
            .with_displayed(line.clone())
            .with_line(line.clone())
            .build();

        let input = ParsedLine::Choice {
            level: 1,
            choice: choice.clone(),
        };

        let mut index = 0;
        let branch = new_parse_single_choice(&mut index, 1, &[input]).unwrap();

        assert_eq!(branch.choice, choice);
    }

    #[test]
    fn parsing_a_single_branch_adds_lines_as_items_until_next_branch_is_encountered() {
        let level = 1;
        let choice = get_empty_choice(level);
        let line = get_parsed_line("");

        let lines = vec![
            choice.clone(),
            line.clone(),
            line.clone(),
            choice.clone(),
            line.clone(),
        ];

        let mut index = 0;
        let first = new_parse_single_choice(&mut index, level, &lines).unwrap();
        let second = new_parse_single_choice(&mut index, level, &lines).unwrap();

        assert_eq!(first.items.len(), 3);
        assert!(first.items[1].is_line());
        assert!(first.items[2].is_line());

        assert_eq!(second.items.len(), 2);
        assert!(second.items[1].is_line());
    }

    // #[test]
    // fn parsing_choices_adds_found_regular_lines_as_items() {
    //     let level = 1;
    //     let choice = get_empty_choice(level);
    //     let line = get_parsed_line("");

    //     let lines = vec![
    //         choice.clone(),
    //         line.clone(),
    //         line.clone(),
    //         choice.clone(),
    //         line.clone(),
    //     ];

    //     let mut index = 0;

    //     let first = parse_single_choice(&mut index, level, &lines).unwrap();
    //     let second = parse_single_choice(&mut index, level, &lines).unwrap();
    //     assert_eq!(index, 5);

    //     assert!(first.is_choice());
    //     assert_eq!(first.len(), 3);
    //     assert!(first.node().items[0].is_line());
    //     assert!(first.node().items[1].is_line());
    //     assert!(first.node().items[2].is_line());

    //     assert!(second.is_choice());
    //     assert_eq!(second.len(), 2);
    //     assert!(second.node().items[0].is_line());
    //     assert!(second.node().items[1].is_line());
    // }

    #[test]
    fn parsing_a_branch_stops_when_lower_level_branch_is_encountered() {
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);

        let lines = vec![choice2.clone(), choice2.clone(), choice1.clone()];

        let current_level = 2;

        let mut index = 0;
        assert!(new_parse_single_choice(&mut index, current_level, &lines).is_some());
        assert!(new_parse_single_choice(&mut index, current_level, &lines).is_some());

        // Here we encounter the level one choice and immediately return
        assert!(new_parse_single_choice(&mut index, current_level, &lines).is_none());

        assert_eq!(index, 2);
    }

    // #[test]
    // fn parsing_choices_stops_when_a_lower_level_choice_is_encountered() {
    //     let choice1 = get_empty_choice(1);
    //     let choice2 = get_empty_choice(2);

    //     let lines = vec![choice2.clone(), choice2.clone(), choice1.clone()];

    //     let mut index = 0;

    //     assert!(parse_single_choice(&mut index, 2, &lines).is_some());
    //     assert!(parse_single_choice(&mut index, 2, &lines).is_some());
    //     assert!(parse_single_choice(&mut index, 2, &lines).is_none());

    //     assert_eq!(index, 2);
    // }

    // #[test]
    // fn parsing_choice_set_returns_all_choices_with_nested_content() {
    //     let choice = get_empty_choice(1);
    //     let line1 = ParsedLine::Line(LineData::from_str("one").unwrap());
    //     let line2 = ParsedLine::Line(LineData::from_str("two").unwrap());
    //     let line3 = ParsedLine::Line(LineData::from_str("three").unwrap());

    //     let lines = vec![
    //         choice.clone(),
    //         line1.clone(),
    //         line2.clone(),
    //         choice.clone(),
    //         line3.clone(),
    //         choice.clone(),
    //     ];

    //     let mut index = 0;

    //     let root = parse_choice_set(&mut index, 1, &lines);

    //     assert_eq!(index, 6);
    //     assert!(root.is_choice_set());

    //     assert_eq!(root.len(), 3);

    //     for item in &root.node().items {
    //         assert!(item.is_choice());
    //     }

    //     assert_eq!(root[0].len(), 3);
    //     assert_eq!(root[1].len(), 2);
    //     assert_eq!(root[2].len(), 1);
    // }

    #[test]
    fn parsing_a_branching_choice_returns_all_branches_with_their_nested_content() {
        let choice = get_empty_choice(1);
        let line1 = ParsedLine::Line(LineData::from_str("one").unwrap());
        let line2 = ParsedLine::Line(LineData::from_str("two").unwrap());
        let line3 = ParsedLine::Line(LineData::from_str("three").unwrap());

        let lines = vec![
            choice.clone(),
            line1.clone(),
            line2.clone(),
            choice.clone(),
            line3.clone(),
            choice.clone(),
        ];

        let mut index = 0;

        let branching_set = new_parse_choice_set(&mut index, 1, &lines);

        assert_eq!(index, 6);

        assert_eq!(branching_set.len(), 3);

        assert_eq!(branching_set[0].items.len(), 3);
        assert_eq!(branching_set[1].items.len(), 2);
        assert_eq!(branching_set[2].items.len(), 1);
    }

    // #[test]
    // fn parsing_choices_turns_nested_choices_into_choice_sets() {
    //     let choice1 = get_empty_choice(1);
    //     let choice2 = get_empty_choice(2);
    //     let lines = vec![choice1.clone(), choice2.clone()];

    //     let mut index = 0;

    //     let root = parse_single_choice(&mut index, 1, &lines).unwrap();

    //     assert!(root.is_choice());
    //     assert_eq!(index, 2);

    //     assert_eq!(root.len(), 2);
    //     assert!(root[1].is_choice_set());
    //     assert!(root[1][0].is_choice());
    // }

    #[test]
    fn higher_level_branches_are_added_as_children_to_branch() {
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);
        let lines = vec![choice1.clone(), choice2.clone()];

        let mut index = 0;

        let branch = new_parse_single_choice(&mut index, 1, &lines).unwrap();

        assert_eq!(branch.items.len(), 2);
        assert!(branch.items[1].is_branching_choice());
    }

    // #[test]
    // fn parsing_choice_set_returns_early_if_lower_level_choice_is_encountered() {
    //     let choice1 = get_empty_choice(1);
    //     let choice2 = get_empty_choice(2);

    //     let lines = vec![choice2.clone(), choice1.clone()];

    //     let mut index = 0;

    //     let root = parse_choice_set(&mut index, 2, &lines);

    //     assert_eq!(index, 1);
    //     assert_eq!(root.len(), 1);
    // }

    // #[test]
    // fn parsing_choice_set_returns_handles_multiple_simultaneous_drops_in_level() {
    //     let choice2 = get_empty_choice(2);
    //     let choice3 = get_empty_choice(3);
    //     let choice4 = get_empty_choice(4);

    //     let lines = vec![
    //         choice2.clone(),
    //         choice3.clone(),
    //         choice4.clone(),
    //         choice2.clone(),
    //     ];

    //     let mut index = 0;

    //     let root = parse_choice_set(&mut index, 2, &lines);

    //     assert_eq!(index, 4);
    //     assert_eq!(root.len(), 2);
    // }

    #[test]
    fn parsing_branching_set_handles_multiple_simultaneous_drops_in_level() {
        let choice2 = get_empty_choice(2);
        let choice3 = get_empty_choice(3);
        let choice4 = get_empty_choice(4);

        let lines = vec![
            choice2.clone(),
            choice3.clone(),
            choice4.clone(),
            choice2.clone(),
        ];

        let mut index = 0;
        let branching_set = new_parse_choice_set(&mut index, 2, &lines);

        assert_eq!(branching_set.len(), 2);
    }

    // #[test]
    // fn parsing_complex_nested_structure_works() {
    //     let choice1 = get_empty_choice(1);
    //     let choice2 = get_empty_choice(2);
    //     let choice3 = get_empty_choice(3);
    //     let choice4 = get_empty_choice(4);

    //     let line = get_parsed_line("");

    //     let lines = vec![
    //         choice1.clone(), // 0
    //         choice2.clone(),
    //         choice2.clone(),
    //         choice1.clone(), // 1
    //         choice2.clone(), //
    //         choice3.clone(),
    //         line.clone(),
    //         line.clone(),
    //         choice4.clone(),
    //         choice1.clone(), // 2
    //     ];

    //     let mut index = 0;

    //     let root = parse_choice_set(&mut index, 1, &lines);

    //     assert_eq!(index, 10);
    //     assert!(root.is_choice_set());

    //     // Assert that the level 3 choice has two lines and one final choice_set
    //     let node = &root[1][1][0][1][0];

    //     assert_eq!(node.len(), 4);
    //     assert!(node[0].is_line());
    //     assert!(node[1].is_line());
    //     assert!(node[2].is_line());
    //     assert!(node[3].is_choice_set());
    // }

    #[test]
    fn parsing_complex_nested_structure_works() {
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);
        let choice3 = get_empty_choice(3);
        let choice4 = get_empty_choice(4);

        let line = get_parsed_line("");

        let lines = vec![
            choice1.clone(), // 0
            choice2.clone(),
            choice2.clone(),
            choice1.clone(), // 1
            // Line from choice: 1.0
            choice2.clone(), // 1.1.0
            // Line from choice: 1.1.0.0
            choice3.clone(), // 1.1.0.1
            line.clone(),
            line.clone(),
            choice4.clone(),
            choice1.clone(), // 0.2
        ];

        let mut index = 0;

        let branching_set = new_parse_choice_set(&mut index, 1, &lines);

        // Assert that the level 3 choice has two lines and one final choice_set
        let branch = {
            match &branching_set[1].items[1] {
                NodeContainer::BranchingChoice(level_two_branches) => {
                    match &level_two_branches[0].items[1] {
                        NodeContainer::BranchingChoice(level_three_branches) => {
                            &level_three_branches[0]
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        };

        assert_eq!(branch.items.len(), 4);
        assert!(branch.items[1].is_line());
        assert!(branch.items[2].is_line());
        assert!(branch.items[3].is_branching_choice());
    }

    // #[test]
    // fn choice_set_wrapper_adds_gather_line_if_present() {
    //     let choice1 = get_empty_choice(1);
    //     let gather1 = get_empty_gather(1);

    //     let lines_without_gather = vec![choice1.clone(), choice1.clone(), choice1.clone()];

    //     let mut index = 0;

    //     let (root, line) = parse_choice_set_with_gather(&mut index, 1, &lines_without_gather);

    //     assert!(root.is_choice_set());
    //     assert_eq!(root.len(), 3);
    //     assert!(line.is_none());

    //     let lines_with_gather = vec![
    //         choice1.clone(),
    //         choice1.clone(),
    //         gather1.clone(),
    //         choice1.clone(),
    //     ];

    //     index = 0;

    //     let (root, line) = parse_choice_set_with_gather(&mut index, 1, &lines_with_gather);

    //     assert!(root.is_choice_set());
    //     assert_eq!(root.len(), 2);
    //     assert!(line.unwrap().is_line());
    // }

    #[test]
    fn branching_choice_set_wrapper_returns_gather_line_separately_if_present() {
        let choice1 = get_empty_choice(1);
        let gather1 = get_empty_gather(1);

        let lines_without_gather = vec![choice1.clone(), choice1.clone(), choice1.clone()];

        let mut index = 0;

        let (_, line) = new_parse_choice_set_with_gather(&mut index, 1, &lines_without_gather);
        assert!(line.is_none());

        let lines_with_gather = vec![
            choice1.clone(),
            choice1.clone(),
            gather1.clone(),
            choice1.clone(),
        ];

        index = 0;

        let (_, line) = new_parse_choice_set_with_gather(&mut index, 1, &lines_with_gather);
        assert!(line.is_some());
    }

    // #[test]
    // fn choice_set_wrapper_increments_index_if_gather() {
    //     let choice1 = get_empty_choice(1);
    //     let gather1 = get_empty_gather(1);

    //     let lines_without_gather = vec![choice1.clone()];

    //     let mut index = 0;

    //     parse_choice_set_with_gather(&mut index, 1, &lines_without_gather);

    //     assert_eq!(index, 1);

    //     let lines_with_gather = vec![choice1.clone(), gather1.clone()];

    //     index = 0;

    //     parse_choice_set_with_gather(&mut index, 1, &lines_with_gather);

    //     assert_eq!(index, 2);
    // }

    #[test]
    fn branching_choice_set_wrapper_increments_the_index_for_found_gathers() {
        let choice1 = get_empty_choice(1);
        let gather1 = get_empty_gather(1);

        let lines_without_gather = vec![choice1.clone()];

        let mut index = 0;

        new_parse_choice_set_with_gather(&mut index, 1, &lines_without_gather);
        assert_eq!(index, 1);

        let lines_with_gather = vec![choice1.clone(), gather1.clone()];

        index = 0;

        new_parse_choice_set_with_gather(&mut index, 1, &lines_with_gather);
        assert_eq!(index, 2);
    }

    // #[test]
    // fn gathers_end_choice_sets_at_same_level() {
    //     let choice1 = get_empty_choice(1);
    //     let gather1 = get_empty_gather(1);

    //     let lines = vec![
    //         choice1.clone(),
    //         choice1.clone(),
    //         gather1.clone(),
    //         choice1.clone(),
    //     ];

    //     let mut index = 0;

    //     let root = parse_choice_set(&mut index, 1, &lines);

    //     assert_eq!(index, 2);
    //     assert_eq!(root.len(), 2);
    // }

    #[test]
    fn gathers_end_branching_choice_sets_at_same_level() {
        let choice1 = get_empty_choice(1);
        let gather1 = get_empty_gather(1);

        let lines = vec![
            choice1.clone(),
            choice1.clone(),
            gather1.clone(),
            choice1.clone(),
        ];

        let mut index = 0;

        let branching_set = new_parse_choice_set(&mut index, 1, &lines);

        assert_eq!(index, 2);
        assert_eq!(branching_set.len(), 2);
    }

    #[test]
    fn multilevel_gather_check() {
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);
        let gather1 = get_empty_gather(1);
        let gather2 = get_empty_gather(2);

        let lines = vec![
            choice1.clone(),
            choice1.clone(),
            choice2.clone(),
            choice2.clone(),
            gather2.clone(),
            choice2.clone(),
            choice2.clone(),
            choice1.clone(),
            gather1.clone(),
            choice1.clone(), // 9
        ];

        let mut index = 0;

        let (branching_set, _) = new_parse_choice_set_with_gather(&mut index, 1, &lines);

        assert_eq!(index, 9);
        assert_eq!(branching_set.len(), 3);
    }

    // #[test]
    // fn parse_node_with_two_initial_items_then_nest_into_a_choice_set() {
    //     let line = get_parsed_line("");
    //     let choice1 = get_empty_choice(1);

    //     let lines = vec![line.clone(), line.clone(), choice1.clone(), choice1.clone()];

    //     let root = parse_full_node(&lines);

    //     assert_eq!(root.items.len(), 3);
    //     assert!(root.items[0].is_line());
    //     assert!(root.items[1].is_line());
    //     assert!(root.items[2].is_choice_set());
    // }

    #[test]
    fn full_node_parsing_starts_by_parsing_lines_before_parsing_branches() {
        let line = get_parsed_line("");
        let choice1 = get_empty_choice(1);

        let lines = vec![line.clone(), line.clone(), choice1.clone(), choice1.clone()];

        let root = new_parse_full_node(&lines);

        assert_eq!(root.items.len(), 3);
        assert!(root.items[0].is_line());
        assert!(root.items[1].is_line());
        assert!(root.items[2].is_branching_choice());
    }

    // #[test]
    // fn parse_node_with_a_gather_adds_line() {
    //     let line = get_parsed_line("");
    //     let choice1 = get_empty_choice(1);
    //     let choice2 = get_empty_choice(2);
    //     let gather1 = get_empty_gather(1);
    //     let gather2 = get_empty_gather(2);

    //     let lines = vec![
    //         line.clone(),
    //         choice1.clone(), // ChoiceSet starts here as second level-1 element
    //         choice2.clone(), // First element of level-2 ChoiceSet
    //         choice2.clone(),
    //         gather2.clone(), // Breaks level-2 ChoiceSet; becomes second element
    //         choice1.clone(),
    //         gather2.clone(),
    //         choice1.clone(),
    //         gather1.clone(), // Breaks ChoiceSet; becomes third level-1 element
    //         choice1.clone(), // Fourth level-1 element due to gather
    //     ];

    //     let root = parse_full_node(&lines);

    //     assert_eq!(root.items.len(), 4);
    //     assert!(root.items[1].is_choice_set());
    //     assert!(root.items[2].is_line());
    //     assert!(root.items[3].is_choice_set());

    //     assert_eq!(root.items[1][0].len(), 3);
    //     assert!(root.items[1][0][2].is_line());
    // }

    #[test]
    fn full_node_parsing_with_gather_adds_the_gather_line_below_the_branch() {
        let line = get_parsed_line("");
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);
        let gather1 = get_empty_gather(1);
        let gather2 = get_empty_gather(2);

        let lines = vec![
            line.clone(),
            choice1.clone(), // ChoiceSet starts here as second level-1 element
            choice2.clone(), // First element of level-2 ChoiceSet
            choice2.clone(),
            gather2.clone(), // Breaks level-2 ChoiceSet; becomes second element
            choice1.clone(),
            gather2.clone(),
            choice1.clone(),
            gather1.clone(), // Breaks ChoiceSet; becomes third level-1 element
        ];

        let root_node = new_parse_full_node(&lines);

        assert_eq!(root_node.items.len(), 3);
        assert!(root_node.items[1].is_branching_choice());
        assert!(root_node.items[2].is_line());
    }

    // #[test]
    // fn parse_empty_list_return_empty_node() {
    //     let root = parse_full_node(&[]);
    //     assert_eq!(root.items.len(), 0);
    // }

    #[test]
    fn parse_empty_list_return_empty_node() {
        let root_node = new_parse_full_node(&[]);
        assert_eq!(root_node.items.len(), 0);
    }

    // #[test]
    // fn parse_list_with_only_choices_works() {
    //     let choice = get_empty_choice(1);
    //     let root = parse_full_node(&[choice.clone(), choice.clone(), choice.clone()]);

    //     assert_eq!(root.items.len(), 1);
    //     assert!(root.items[0].is_choice_set());
    //     assert_eq!(root.items[0].len(), 3);
    // }

    #[test]
    fn parse_list_with_only_branches_works() {
        let choice = get_empty_choice(1);
        let root = new_parse_full_node(&[choice.clone(), choice.clone(), choice.clone()]);

        assert_eq!(root.items.len(), 1);
        match &root.items[0] {
            NodeContainer::BranchingChoice(branches) => {
                assert_eq!(branches.len(), 3);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn parse_list_with_non_matched_gathers_turns_them_into_lines() {
        let gather = get_empty_gather(1);
        let root = new_parse_full_node(&[gather.clone(), gather.clone(), gather.clone()]);

        assert_eq!(root.items.len(), 3);

        for item in root.items {
            assert!(item.is_line());
        }
    }

    // #[test]
    // fn parse_list_with_non_matched_gathers_turns_them_into_lines() {
    //     let gather = get_empty_gather(1);
    //     let root = parse_full_node(&[gather.clone(), gather.clone(), gather.clone()]);

    //     assert_eq!(root.items.len(), 3);

    //     for item in root.items {
    //         assert!(item.is_line());
    //     }
    // }

    // #[test]
    // fn parse_list_with_high_leveled_choices_still_just_nests_them() {
    //     let choice1 = get_empty_choice(64);
    //     let choice2 = get_empty_choice(128);

    //     let root = parse_full_node(&[
    //         choice1.clone(),
    //         choice1.clone(),
    //         choice2.clone(),
    //         choice2.clone(),
    //         choice1.clone(),
    //     ]);

    //     assert_eq!(root.items.len(), 1);
    //     assert_eq!(root.items[0].len(), 3);
    //     assert_eq!(root.items[0][1][1].len(), 2);
    // }

    #[test]
    fn parse_list_with_high_leveled_branches_still_just_nests_them() {
        let choice1 = get_empty_choice(64);
        let choice2 = get_empty_choice(128);

        let root_node = new_parse_full_node(&[
            choice1.clone(),
            choice1.clone(),
            choice2.clone(),
            choice2.clone(),
            choice1.clone(),
        ]);

        assert_eq!(root_node.items.len(), 1);

        match &root_node.items[0] {
            NodeContainer::BranchingChoice(branches) => {
                assert_eq!(branches.len(), 3);

                match &branches[1].items[1] {
                    NodeContainer::BranchingChoice(nested_branches) => {
                        assert_eq!(nested_branches.len(), 2);
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn get_empty_choice(level: u32) -> ParsedLine {
        let choice = ChoiceData::empty();
        ParsedLine::Choice { level, choice }
    }

    pub fn get_empty_gather(level: u32) -> ParsedLine {
        let line = LineData::from_str("").unwrap();

        ParsedLine::Gather { level, line }
    }

    pub fn get_parsed_line(s: &str) -> ParsedLine {
        ParsedLine::Line(LineData::from_str(s).unwrap())
    }
}
