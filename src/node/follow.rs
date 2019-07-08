use crate::{
    error::{
        BadGraphKind, FollowError, IncorrectStackKind, InternalError, NodeItemKind, WhichIndex,
    },
    line::{Choice, LineKind},
    story::{FollowResult, LineBuffer, Next},
};

pub type Stack = Vec<usize>;

use super::node::{DialogueNode, NodeItem, NodeType};

impl DialogueNode {
    /// Walk through a `DialogueNode` graph tree recursively and add all lines to the buffer
    /// until a `Divert` to a different knot or a set of choices to the user is found.
    ///
    /// The index of the current item is set in the `Stack` at the current location index.
    /// This is to keep track of from where (relative to the root of the tree) a set of
    /// choices was found, so that we can return to it with `follow_with_choice`.
    ///
    /// If an index for the current level is set the items are read starting from that index.
    /// It is otherwise initialized to 0.
    ///
    /// Additionally, the input `Stack` may not be longer than the current level. This is to
    /// ensure that the call stack is always what we expect it to be when walking through
    /// a graph.
    pub fn follow(
        &self,
        current_level: usize,
        buffer: &mut LineBuffer,
        stack: &mut Stack,
    ) -> FollowResult {
        let index = add_or_get_mut_stack_index_for_level(current_level, stack)?;

        while *index < self.items.len() {
            let item = &self.items[*index];
            *index += 1;

            match item {
                NodeItem::Line(line) => {
                    buffer.push(line.clone());

                    if let LineKind::Divert(destination) = &line.kind {
                        return Ok(Next::Divert(destination.clone()));
                    }
                }
                NodeItem::Node {
                    kind: NodeType::ChoiceSet,
                    ..
                } => {
                    let choices = get_choices_from_set(item, current_level)?;

                    // Revert the stack index to the current location, since we will continue
                    // from here after selecting a choice.
                    *index -= 1;

                    return Ok(Next::ChoiceSet(choices));
                }
                _ => eprintln!(
                    "warning: encountered {:?} when following a node, \
                     which should not happen (node: {:?}, index: {})",
                    &item,
                    &self,
                    *index - 1
                ),
            }
        }

        Ok(Next::Done)
    }

    /// Follow a `Stack` to the latest `DialogueNode`, which returned a set of choices
    /// for the user. Continue from that set of choices by selecting the item with
    /// given index `choice` and calling `follow` on its node to continue through the story.
    ///
    /// As the call returns from that `Choice` node, continue `follow`ing through the node
    /// in which the set of choices was found.
    pub fn follow_with_choice(
        &self,
        choice: usize,
        current_level: usize,
        buffer: &mut LineBuffer,
        stack: &mut Stack,
    ) -> FollowResult {
        let result = if current_level < stack.len() - 1 {
            let next_level_node = self.follow_stack_to_next_choice(current_level, None, stack)?;

            next_level_node.follow_with_choice(choice, current_level + 2, buffer, stack)
        } else {
            let choice_node =
                self.follow_stack_to_next_choice(current_level, Some(choice), stack)?;

            stack.push(choice);

            choice_node.follow(current_level + 2, buffer, stack)
        }?;

        match &result {
            Next::Done => {
                // Continue reading the current node's items
                stack.truncate(current_level + 1);
                stack[current_level] += 1;
                self.follow(current_level, buffer, stack)
            }
            _ => Ok(result),
        }
    }

    /// Follow through the stack and retrieve the next `DialogueNode` which will be a `Choice`.
    ///
    /// To get the choice, the index of the `ChoiceSet` child item is read from the stack
    /// at the current level. Then the `Choice` child item of that set is read by either
    /// the `with_choice` index if given, or if not by reading the stack.
    ///
    /// This makes the following assumptions:
    ///  * The current node has a `ChoiceSet` at index stack[current_level]
    ///  * If `with_choice` is none, stack[current_level + 1] is present to get the index
    ///  * The `ChoiceSet` has `Choice` at that index
    ///
    /// If any of those assumptions are not true, the stack does not represent the current
    /// `DialogueTree` as constructed from the root or `follow_with_choice` was called
    /// with an incorrect choice. An error will be returned.
    fn follow_stack_to_next_choice(
        &self,
        current_level: usize,
        with_choice: Option<usize>,
        stack: &Stack,
    ) -> Result<&DialogueNode, FollowError> {
        let choice_set_index = get_stack_index_for_level(current_level, stack, WhichIndex::Parent)?;
        let choice_index = match with_choice {
            Some(index) => index,
            None => get_stack_index_for_level(current_level + 1, stack, WhichIndex::Child)?,
        };

        let node_item = self.get_node_item(choice_set_index, current_level, stack)?;
        let choice_set_node = get_choice_set_node(node_item, choice_set_index, current_level)?;

        let choice_item = choice_set_node
            .get_node_item(choice_index, current_level + 1, stack)
            .map_err(|err| {
                if with_choice.is_some() {
                    FollowError::InvalidChoice {
                        selection: choice_index,
                        num_choices: choice_set_node.items.len(),
                    }
                } else {
                    err.into()
                }
            })?;

        get_choice_node(choice_item, choice_index, current_level + 1).map_err(|err| err.into())
    }

    // Safely get the `NodeItem` at given index.
    fn get_node_item(
        &self,
        index: usize,
        current_level: usize,
        stack: &Stack,
    ) -> Result<&NodeItem, InternalError> {
        self.items.get(index).ok_or(InternalError::IncorrectStack {
            kind: IncorrectStackKind::BadIndices {
                node_level: current_level,
                index: index,
                num_items: self.items.len(),
            },
            stack: stack.clone(),
        })
    }
}

/// Safely get the the stack index at the current level, either for a parent or child node (which
/// indicates level + 1 when tracing the error).
fn get_stack_index_for_level(
    level: usize,
    stack: &Stack,
    kind: WhichIndex,
) -> Result<usize, InternalError> {
    stack
        .get(level)
        .cloned()
        .ok_or(InternalError::IncorrectStack {
            kind: IncorrectStackKind::MissingIndices {
                node_level: level,
                kind,
            },
            stack: stack.clone(),
        })
}

/// Get a mutable stack index for the current level. If the current stack has no entry
/// for the current level, add and return it starting from 0. Return an error if the
/// stack is incomplete, either by not containing all previous stack entries for nodes
/// or if it has not been truncated to the current node level.
fn add_or_get_mut_stack_index_for_level(
    current_level: usize,
    stack: &mut Stack,
) -> Result<&mut usize, InternalError> {
    if stack.len() < current_level {
        return Err(InternalError::IncorrectStack {
            kind: IncorrectStackKind::Gap {
                node_level: current_level,
            },
            stack: stack.clone(),
        });
    } else if stack.len() > current_level + 1 {
        return Err(InternalError::IncorrectStack {
            kind: IncorrectStackKind::NotTruncated {
                node_level: current_level,
            },
            stack: stack.clone(),
        });
    }

    if stack.len() == current_level {
        stack.push(0);
    }

    Ok(stack.last_mut().unwrap())
}

/// Get the `DialogueNode` from a `NodeItem` of `NodeType::ChoiceSet`.
fn get_choice_set_node(
    item: &NodeItem,
    index: usize,
    node_level: usize,
) -> Result<&DialogueNode, InternalError> {
    match &item {
        NodeItem::Node {
            kind: NodeType::ChoiceSet,
            node,
        } => Ok(node),
        _ => Err(InternalError::BadGraph(BadGraphKind::ExpectedNode {
            index,
            node_level,
            expected: NodeItemKind::ChoiceSet,
            found: item.into(),
        })),
    }
}

/// Get the `DialogueNode` from a `NodeItem` of `NodeType::Choice`.
fn get_choice_node(
    item: &NodeItem,
    index: usize,
    node_level: usize,
) -> Result<&DialogueNode, InternalError> {
    match &item {
        NodeItem::Node {
            kind: NodeType::Choice(..),
            node,
        } => Ok(node),
        _ => Err(InternalError::BadGraph(BadGraphKind::ExpectedNode {
            index,
            node_level,
            expected: NodeItemKind::Choice,
            found: item.into(),
        })),
    }
}

// This should only ever be called on a `NodeItem` which is a `ChoiceSet`. Since we
// are calling this internally this should only ever be the case.
fn get_choices_from_set(
    choice_set: &NodeItem,
    node_level: usize,
) -> Result<Vec<Choice>, InternalError> {
    match choice_set {
        NodeItem::Node {
            kind: NodeType::ChoiceSet,
            node,
        } => node
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| get_choice_from_node_item(item, index, node_level))
            .collect(),
        _ => Err(InternalError::BadGraph(BadGraphKind::ExpectedNode {
            index: 0,
            node_level,
            expected: NodeItemKind::ChoiceSet,
            found: choice_set.into(),
        })),
    }
}

// This should only ever be called on a `NodeItem` which is a `Choice`. Since we
// are calling this internally this should only ever be the case.
fn get_choice_from_node_item(
    item: &NodeItem,
    index: usize,
    node_level: usize,
) -> Result<Choice, InternalError> {
    match item {
        NodeItem::Node {
            kind: NodeType::Choice(choice),
            ..
        } => Ok(choice.clone()),
        _ => Err(InternalError::BadGraph(BadGraphKind::ExpectedNode {
            index,
            node_level,
            expected: NodeItemKind::Choice,
            found: item.into(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        ops::{Index, IndexMut},
        str::FromStr,
    };

    use crate::line::Line;

    #[test]
    fn follow_a_pure_line_node_adds_lines_to_buffer() {
        let (line1, item1) = get_line_and_node_item_line("Hello, World!");
        let (line2, item2) = get_line_and_node_item_line("Hello?");
        let (line3, item3) = get_line_and_node_item_line("Hello, is anyone there?");

        let node = DialogueNode {
            items: vec![item1, item2, item3],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        let result = node.follow(0, &mut buffer, &mut stack).unwrap();

        match result {
            Next::Done => (),
            Next::Divert(..) => panic!("node should be `Done` but was `Divert`"),
            Next::ChoiceSet(..) => panic!("node should be `Done` but was `ChoiceSet`"),
        }

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0], line1);
        assert_eq!(buffer[1], line2);
        assert_eq!(buffer[2], line3);
    }

    #[test]
    fn follow_a_node_pushes_last_index_to_stack() {
        let item = get_node_item_line("");

        let node = DialogueNode {
            items: vec![item.clone(), item],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        node.follow(0, &mut buffer, &mut stack).unwrap();

        assert_eq!(&stack, &[2]);
    }

    #[test]
    fn follow_begins_at_index_from_stack_if_present_for_level_else_creates_it_from_zero() {
        let (line1, item1) = get_line_and_node_item_line("Hello, World!");
        let (_, item2) = get_line_and_node_item_line("Hello?");
        let (line3, item3) = get_line_and_node_item_line("Hello, is anyone there?");

        let node = DialogueNode {
            items: vec![item1, item2, item3],
        };

        let level = 5;

        let mut buffer = Vec::new();
        let mut stack = vec![0; level];

        // Index not present in stack
        node.follow(level, &mut buffer, &mut stack).unwrap();
        assert_eq!(stack, &[0, 0, 0, 0, 0, 3]);
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0], line1);

        // Index present in stack
        buffer.clear();
        let start_index = 2;
        stack[level] = start_index; // Begin from index 2

        node.follow(level, &mut buffer, &mut stack).unwrap();
        assert_eq!(stack, &[0, 0, 0, 0, 0, 3]);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], line3);
    }

    #[test]
    fn follow_returns_error_if_input_stack_is_longer_than_for_the_current_level() {
        let node = DialogueNode { items: Vec::new() };

        let mut buffer = Vec::new();
        let mut stack = vec![0, 0, 0];

        // Index not present in stack
        assert!(node.follow(1, &mut buffer, &mut stack).is_err());
    }

    #[test]
    fn follow_returns_error_if_stack_max_index_and_current_level_differs_by_2_or_more() {
        let node = DialogueNode { items: Vec::new() };

        let mut buffer = Vec::new();

        let mut stack = vec![0; 5]; // max index: 4
        assert!(node.follow(4, &mut buffer, &mut stack).is_ok());

        stack = vec![0; 5];
        assert!(node.follow(5, &mut buffer, &mut stack).is_ok());

        stack = vec![0; 5];
        assert!(node.follow(6, &mut buffer, &mut stack).is_err());
    }

    #[test]
    fn follow_line_returns_early_with_divert() {
        let item = get_node_item_line("Hello, World!");

        let destination = "to_node";
        let divert = LineKind::Divert(destination.to_string());

        let mut line_divert = Line::from_str("").unwrap();
        line_divert.kind = divert.clone();

        let item_divert = get_node_item_with_line(&line_divert);

        let node = DialogueNode {
            items: vec![item.clone(), item.clone(), item_divert, item.clone()],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        match node.follow(0, &mut buffer, &mut stack).unwrap() {
            Next::Divert(result) => assert_eq!(result, destination),
            _ => panic!("node should return a `Divert` but did not"),
        }
    }

    #[test]
    fn follow_line_with_divert_sets_stack_index_to_line_after_divert() {
        let item = get_node_item_line("Hello, World!");

        let destination = "to_node";
        let divert = LineKind::Divert(destination.to_string());

        let mut line_divert = Line::from_str("").unwrap();
        line_divert.kind = divert.clone();

        let item_divert = get_node_item_with_line(&line_divert);

        let node = DialogueNode {
            items: vec![item.clone(), item_divert, item.clone(), item.clone()],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        node.follow(0, &mut buffer, &mut stack).unwrap();

        assert_eq!(stack[0], 2);
    }

    #[test]
    fn follow_line_with_divert_adds_divert_line_text_to_buffer_before_returning() {
        let item = get_node_item_line("Hello, World!");

        let destination = "to_node";
        let divert = LineKind::Divert(destination.to_string());
        let divert_text = "They moved on to the next scene";

        let mut line_divert = Line::from_str(divert_text).unwrap();
        line_divert.kind = divert.clone();

        let item_divert = get_node_item_with_line(&line_divert);

        let node = DialogueNode {
            items: vec![item.clone(), item_divert, item.clone(), item.clone()],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        node.follow(0, &mut buffer, &mut stack).unwrap();

        assert_eq!(buffer[1], line_divert);
    }

    #[test]
    fn follow_into_a_choice_set_returns_the_set() {
        let item = get_node_item_line("Hello, World!");

        let num_choices = 3;
        let choice_set = get_choice_set_with_empty_choices(num_choices);
        let choice_set_copy = get_choice_set_with_empty_choices(num_choices);

        let node = DialogueNode {
            items: vec![item.clone(), choice_set],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        match node.follow(0, &mut buffer, &mut stack).unwrap() {
            Next::ChoiceSet(items) => {
                assert_eq!(items.len(), num_choices);

                for (choice_result, choice_input) in
                    items.iter().zip(choice_set_copy.node().items.iter())
                {
                    assert_eq!(choice_result, choice_input.choice());
                }
            }
            _ => panic!("expected a returned `ChoiceSet` when encountering one but did not get it"),
        }

        assert_eq!(
            stack[0], 1,
            "stack was not set to after the `ChoiceSet` during follow"
        );
    }

    #[test]
    fn follow_with_choice_descends_into_that_node_with_a_follow() {
        let mut choice_set = get_choice_set_with_empty_choices(2);
        let item1 = get_node_item_line("Hello, world!");

        let line2 = Line::from_str("Hello?").unwrap();
        let item2 = get_node_item_with_line(&line2);

        let line3 = Line::from_str("Hello, is anyone there?").unwrap();
        let item3 = get_node_item_with_line(&line3);

        // Add two lines to second choice in the set
        choice_set[1].node_mut().items.push(item2.clone());
        choice_set[1].node_mut().items.push(item3.clone());

        let node = DialogueNode {
            items: vec![item1.clone(), choice_set],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        match node.follow(0, &mut buffer, &mut stack).unwrap() {
            Next::ChoiceSet(..) => (),
            _ => panic!("after following a `Next::ChoiceSet` was expected, but it wasn't"),
        }

        node.follow_with_choice(1, stack.len() - 1, &mut buffer, &mut stack)
            .unwrap();

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[1], line2);
        assert_eq!(buffer[2], line3);
    }

    #[test]
    fn follow_with_choice_updates_stack_when_descending() {
        let item1 = get_node_item_line("Hello, world!");
        let mut choice_set = get_choice_set_with_empty_choices(5);

        let choice = 3;

        // Add another ChoiceSet to the used choice
        choice_set[choice]
            .node_mut()
            .items
            .push(get_choice_set_with_empty_choices(1));

        let node = DialogueNode {
            items: vec![item1, choice_set],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        match node.follow(0, &mut buffer, &mut stack).unwrap() {
            Next::ChoiceSet(..) => (),
            _ => panic!("after following a `Next::ChoiceSet` was expected, but it wasn't"),
        }

        assert_eq!(&stack, &[1]);

        node.follow_with_choice(choice, 0, &mut buffer, &mut stack)
            .unwrap();

        assert_eq!(stack, &[1, choice, 0]);
    }

    #[test]
    fn follow_with_choice_pops_stack_when_returning_from_choice_set() {
        let item = get_node_item_line("Hello, world!");

        let node = DialogueNode {
            items: vec![
                item,
                get_choice_set_with_empty_choices(1),
                get_choice_set_with_empty_choices(1),
            ],
        };

        let mut buffer = Vec::new();
        let mut stack = Vec::new();

        match node.follow(0, &mut buffer, &mut stack).unwrap() {
            Next::ChoiceSet(..) => (),
            _ => panic!("after following a `Next::ChoiceSet` was expected, but it wasn't"),
        }

        assert_eq!(&stack, &[1]);

        node.follow_with_choice(0, 0, &mut buffer, &mut stack)
            .unwrap();

        assert_eq!(&stack, &[2]);
    }

    #[test]
    fn follow_with_choice_goes_through_stack_before_selecting() {
        let item = get_node_item_line("Hello, world!");
        let (line_target, item_target) = get_line_and_node_item_line("Hello, to you too!");

        let mut stack = vec![1, 1, 1]; // `ChoiceSet` at 1, `Choice` 1, then `ChoiceSet` at 1

        // Choice will be at stack location (1, 1, 1) + choice 1 with a unique line
        let mut target_choice_set = get_choice_set_with_empty_choices(3);
        target_choice_set[1]
            .node_mut()
            .items
            .push(item_target.clone());

        // At neighbouring indices (1, 1, 0), (1, 1, 2) we have choice sets with other
        // lines to ensure that we nested into the correct one
        let mut target_siblings_choice_set1 = get_choice_set_with_empty_choices(3);
        target_siblings_choice_set1[1]
            .node_mut()
            .items
            .push(item.clone());
        let mut target_siblings_choice_set2 = get_choice_set_with_empty_choices(3);
        target_siblings_choice_set2[1]
            .node_mut()
            .items
            .push(item.clone());

        // This is the choice set container at stack location (1)
        let mut container_choice_set = get_choice_set_with_empty_choices(3);
        container_choice_set[1]
            .node_mut()
            .items
            .push(target_siblings_choice_set1);
        container_choice_set[1]
            .node_mut()
            .items
            .push(target_choice_set);
        container_choice_set[1]
            .node_mut()
            .items
            .push(target_siblings_choice_set2);

        let node = DialogueNode {
            items: vec![item.clone(), container_choice_set, item.clone()],
        };

        let mut buffer = Vec::new();

        match node
            .follow_with_choice(1, 0, &mut buffer, &mut stack)
            .unwrap()
        {
            Next::ChoiceSet(..) => (),
            _ => panic!("after following a `Next::ChoiceSet` was expected, but it wasn't"),
        }

        assert_eq!(
            buffer.len(),
            1,
            "buffer after nested follow does not contain the right number of lines"
        );
        assert_eq!(
            buffer[0], line_target,
            "buffer after nested follow does not contain the target line"
        );
    }

    #[test]
    fn after_follow_with_choice_returns_previous_levels_continue_through_their_children() {
        let (line1, item1) = get_line_and_node_item_line("Hello, world!");
        let (line2, item2) = get_line_and_node_item_line("Hello, to you too!");

        let mut stack = vec![0, 0, 0];

        let target_choice_set = get_choice_set_with_empty_choices(3);
        let mut container_choice_set = get_choice_set_with_empty_choices(3);

        // Add target choice set to container
        container_choice_set[0]
            .node_mut()
            .items
            .push(target_choice_set);

        // After choice set there is a line that should be added when the choice returns
        container_choice_set[0].node_mut().items.push(item1);

        let node = DialogueNode {
            items: vec![
                container_choice_set,
                // After the first choice set there is a second line that should be added
                item2,
            ],
        };

        let mut buffer = Vec::new();

        match node
            .follow_with_choice(0, 0, &mut buffer, &mut stack)
            .unwrap()
        {
            Next::Done => (),
            _ => panic!("after following `Next::Done` was expected, but it wasn't"),
        }

        assert_eq!(buffer, &[line1, line2]);
    }

    #[test]
    fn follow_stack_to_next_choice_returns_next_choice_node_from_stack() {
        let stack = vec![0, 1]; // Get second choice in set

        let item1 = get_node_item_line("Hello, World!");
        let (line2, item2) = get_line_and_node_item_line("Hello?");

        let mut choice_set = get_choice_set_with_empty_choices(2);

        choice_set[0].node_mut().items.push(item1.clone());

        choice_set[1].node_mut().items.push(item2.clone());

        let root = DialogueNode {
            items: vec![choice_set],
        };

        let node = root.follow_stack_to_next_choice(0, None, &stack).unwrap();

        assert_eq!(node.items.len(), 1);
        assert_eq!(node.items[0].line(), &line2);
    }

    #[test]
    fn follow_stack_to_next_choice_gets_stack_index_for_given_level() {
        let current_level = 2;

        // 10 is out of bounds, but the current level will give us in bounds indices
        let stack = vec![10, 10, 0, 1];

        let (line, item) = get_line_and_node_item_line("Hello, World!");

        let mut choice_set = get_choice_set_with_empty_choices(2);

        choice_set[1].node_mut().items.push(item.clone());

        let root = DialogueNode {
            items: vec![choice_set],
        };

        let node = root
            .follow_stack_to_next_choice(current_level, None, &stack)
            .unwrap();

        assert_eq!(node.items.len(), 1);
        assert_eq!(node.items[0].line(), &line);
    }

    #[test]
    fn follow_stack_to_next_choice_returns_next_choice_node_from_given_choice() {
        // Same as last test, but while the stack index says to return choice 1,
        // the explicit choice says 0.
        let stack = vec![0, 1];

        let (line1, item1) = get_line_and_node_item_line("Hello, World!");
        let item2 = get_node_item_line("Hello?");

        let mut choice_set = get_choice_set_with_empty_choices(2);

        choice_set[0].node_mut().items.push(item1.clone());

        choice_set[1].node_mut().items.push(item2.clone());

        let root = DialogueNode {
            items: vec![choice_set],
        };

        let node = root
            .follow_stack_to_next_choice(0, Some(0), &stack)
            .unwrap();

        assert_eq!(node.items.len(), 1);
        assert_eq!(node.items[0].line(), &line1);
    }

    #[test]
    fn follow_with_user_choice_not_in_the_set_returns_invalid_choice_error() {
        let choice = 2;
        let choice_set = get_choice_set_with_empty_choices(choice);

        let node = DialogueNode {
            items: vec![choice_set],
        };

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        match node.follow_with_choice(choice, 0, &mut buffer, &mut stack) {
            Err(FollowError::InvalidChoice { .. }) => (),
            _ => panic!("`FollowError::InvalidChoice` was not yielded"),
        }
    }

    /***************************************************
     * Helper functions to do test assertion and debug *
     ***************************************************/

    impl DialogueNode {
        // Return a string representation of the entire graph of nodes.
        pub fn display(&self) -> String {
            let mut buffer = String::new();

            for item in &self.items {
                item.display_indent(&mut buffer, 0);
            }

            buffer
        }
    }

    impl NodeItem {
        // If this is another node (`NodeItem::Node`), return a string representation
        // of the entire graph spawning from it.
        pub fn display(&self) -> String {
            let mut buffer = String::new();

            self.display_indent(&mut buffer, 0);

            buffer
        }

        // Recursively descend into children, writing their structure into the buffer
        // with indents added for every level.
        pub fn display_indent(&self, buffer: &mut String, level: usize) {
            let indent = format!("{:width$}", ' ', width = 4 * level);

            match self {
                NodeItem::Line(line) => {
                    let s = format!("{indent}Line(\"{}\")\n", &line.text, indent = indent);
                    buffer.push_str(&s);
                }

                NodeItem::Node { kind, node } => {
                    let variant = match kind {
                        NodeType::ChoiceSet => "ChoiceSet [\n",
                        NodeType::Choice(..) => "Choice [\n",
                    };

                    let s = format!("{indent}{}", variant, indent = indent);
                    buffer.push_str(&s);

                    for item in &node.items {
                        item.display_indent(buffer, level + 1);
                    }

                    let s = format!("{indent}]\n", indent = indent);
                    buffer.push_str(&s);
                }
            }
        }

        // If `Self` is `NodeItem::Node`, return the length of its direct children.
        // Panics if `Self` is not `Node`.
        pub fn len(&self) -> usize {
            match self {
                NodeItem::Node { node, .. } => node.items.len(),
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }

        // If `Self` is `NodeItem::Node` and kind is `NodeType::Choice`, return the choice.
        // Panics if `Self` is otherwise.
        pub fn choice(&self) -> &Choice {
            match self {
                NodeItem::Node {
                    kind: NodeType::Choice(choice),
                    ..
                } => &choice,
                _ => panic!(
                    "expected a `Node` with kind `NodeType::Choice` but found {:?}",
                    self
                ),
            }
        }

        // If `Self` is `NodeItem::Line`, return the boxed line.
        // Panics if `Self` is not `Line`.
        pub fn line(&self) -> &Line {
            match self {
                NodeItem::Line(line) => &line,
                _ => panic!("expected a `Line` but found {:?}", self),
            }
        }

        // If `Self` is `NodeItem::Node`, return the boxed `Node`.
        // Panics if `Self` is not `Node`.
        pub fn node(&self) -> &DialogueNode {
            match self {
                NodeItem::Node { node, .. } => &node,
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }

        // If `Self` is `NodeItem::Node`, return the boxed `Node`.
        // Panics if `Self` is not `Node`.
        pub fn node_mut(&mut self) -> &mut DialogueNode {
            match self {
                NodeItem::Node { ref mut node, .. } => node,
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }

        // Return `true` if `Self` is both `NodeItem::Node` and its kind is `NodeType::Choice`.
        pub fn is_choice(&self) -> bool {
            match self {
                NodeItem::Node {
                    kind: NodeType::Choice(..),
                    ..
                } => true,
                _ => false,
            }
        }

        // Return `true` if `Self` is `NodeItem::Line`.
        pub fn is_line(&self) -> bool {
            match self {
                NodeItem::Line(..) => true,
                _ => false,
            }
        }

        // Return `true` if `Self` is both `NodeItem::Node` and its kind is `NodeType::ChoiceSet`.
        pub fn is_choice_set(&self) -> bool {
            match self {
                NodeItem::Node {
                    kind: NodeType::ChoiceSet,
                    ..
                } => true,
                _ => false,
            }
        }
    }

    // Implement cloning for `NodeItem::Line` objects.
    impl Clone for NodeItem {
        fn clone(&self) -> Self {
            match self {
                NodeItem::Line(line) => NodeItem::Line(line.clone()),
                _ => panic!("tried to clone a `Node` which is not implemented"),
            }
        }
    }

    // If `Self` is `NodeItem::Node`, return the child item with given index.
    // Panics if `Self` is not `Node`.
    impl Index<usize> for NodeItem {
        type Output = Self;

        fn index(&self, index: usize) -> &Self::Output {
            match self {
                NodeItem::Node { node, .. } => &node.items[index],
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }
    }

    // If `Self` is `NodeItem::Node`, return the child item with given index.
    // Panics if `Self` is not `Node`.
    impl IndexMut<usize> for NodeItem {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            match self {
                NodeItem::Node { node, .. } => &mut node.items[index],
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }
    }

    pub fn get_node_item_line(s: &str) -> NodeItem {
        NodeItem::Line(Line::from_str(s).unwrap())
    }

    pub fn get_node_item_with_line(line: &Line) -> NodeItem {
        NodeItem::Line(line.clone())
    }

    pub fn get_line_and_node_item_line(s: &str) -> (Line, NodeItem) {
        let line = Line::from_str(s).unwrap();
        let item = NodeItem::Line(line.clone());

        (line, item)
    }

    pub fn get_choice_set_with_empty_choices(num: usize) -> NodeItem {
        let empty_choice = Choice {
            selection_text: String::new(),
            line: Line::from_str("").unwrap(),
        };

        let items = (0..num)
            .map(|_| NodeItem::Node {
                kind: NodeType::Choice(empty_choice.clone()),
                node: Box::new(DialogueNode { items: Vec::new() }),
            })
            .collect::<Vec<_>>();

        let node = DialogueNode { items };

        NodeItem::Node {
            kind: NodeType::ChoiceSet,
            node: Box::new(node),
        }
    }
}
