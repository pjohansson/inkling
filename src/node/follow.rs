//! Processing nested story content by following, or walking through, it.

use crate::{
    error::{IncorrectNodeStackError, InternalError},
    follow::{ChoiceInfo, EncounteredEvent, FollowResult, LineDataBuffer},
    node::{Branch, NodeItem, RootNode},
};

use std::slice::IterMut;

/// Represents the current stack of choices made from the tree root.
///
/// # Example
///
/// For example, for this tree:
///
/// Root
/// ```text
/// Line
/// Line
/// Branching Set
///     Branch 1
///     Branch 2
///         Line
///         Branching Set   <---- the user is here in the branched story
///             Branch 1
///                 ...
///             Branch 2
///                 ...
///     Branch 3
///         ...
/// ```
///
/// the current stack is [2, 1, 1]. When the user picks a choice the stack is used to
/// advance to the position of that choice set in the tree, then follow from there on.
///
/// Do note that every `Branch` adds a line of text to its children. Lines after this
/// choice start at index 1.
pub type Stack = Vec<usize>;

/// Trait which enables us to walk through the tree of content in a `Stitch`.
///
/// This trait is implemented on all constituent parts (nodes) of the tree. For every line
/// of content in the current node the text is processed and added to a supplied buffer.
///
/// When a branching choice is encountered it is returned and the story will halt until
/// the user supplies a branch to keep following the story from.
pub trait Follow: FollowInternal {
    /// Follow the content of the current node.
    ///
    /// The follow continues until the node runs out of content or a branching choice
    /// is encountered. This node should be the currently active node, representing
    /// the last stack position in a tree.
    ///
    /// The follow will resume from and update the current `Stack` as it walks through
    /// the node.
    ///
    /// # Notes
    ///  *  The method assumes that the last index of the stack belongs to this node,
    ///     since a `follow` will always be called on the deepest level that has been
    ///     reached in the tree.
    ///
    ///     Ensure that the stack is maintained before calling this method.
    fn follow(&mut self, stack: &mut Stack, buffer: &mut LineDataBuffer) -> FollowResult {
        let at_index = stack
            .last_mut()
            .ok_or(InternalError::from(IncorrectNodeStackError::EmptyStack))?;

        if *at_index > self.get_num_items() {
            return Err(InternalError::from(IncorrectNodeStackError::OutOfBounds {
                stack_index: stack.len() - 1,
                stack: stack.clone(),
                num_items: self.get_num_items(),
            })
            .into());
        } else if *at_index == 0 {
            self.increment_num_visited();
        }

        for item in self.iter_mut_items().skip(*at_index) {
            *at_index += 1;

            match item {
                NodeItem::Line(line) => {
                    let result = line
                        .process(buffer)
                        .map_err(|err| InternalError::from(err))?;

                    if let EncounteredEvent::Divert(..) = result {
                        return Ok(result);
                    }
                }
                NodeItem::BranchingPoint(branches) => {
                    *at_index -= 1;

                    let branching_choice_set = get_choices_from_branching_set(branches);

                    return Ok(EncounteredEvent::BranchingChoice(branching_choice_set));
                }
            }
        }

        Ok(EncounteredEvent::Done)
    }

    /// Resume the follow of content in the tree with a supplied choice.
    ///
    /// Will fast forward through the tree to reach the node where the choice was encountered.
    /// The `Stack` is used to accomplish this. The last index in the stack represents
    /// the `NodeItem` of the nested node where the choice was encountered. We advance to
    /// that node from a lower level by checking whether the current `stack_index` represents
    /// this level.
    ///
    /// If the `stack_index` is lower than the stack length - 1 we are not yet at the level
    /// in the tree where the choice was encountered. We recursively move to the next node
    /// by following the stack to it, updating the `stack_index` value when calling it
    /// until we reach the deepest level.
    ///
    /// When reaching the deepest level, `follow` is called on the selected branch of
    /// the choices. Diverts and new branching choices are returned through the stack
    /// if encountered.
    ///
    /// Finally, when we return from a deeper level due to running out of content in that node,
    /// we keep `follow`ing the content in the current node until its end.
    fn follow_with_choice(
        &mut self,
        selection: usize,
        stack_index: usize,
        stack: &mut Stack,
        buffer: &mut LineDataBuffer,
    ) -> FollowResult {
        let result = if let Some(next_branch) = self.get_next_level_branch(stack_index, stack)? {
            next_branch.follow_with_choice(selection, stack_index + 2, stack, buffer)
        } else {
            let selected_branch = self.get_selected_branch(selection, stack_index, stack)?;

            stack.extend_from_slice(&[selection, 0]);

            selected_branch.follow(stack, buffer)
        }?;

        match result {
            EncounteredEvent::Done => {
                stack.truncate(stack_index + 1);
                stack.last_mut().map(|i| *i += 1);

                self.follow(stack, buffer)
            }
            other => Ok(other),
        }
    }
}

impl Follow for RootNode {}
impl Follow for Branch {}

/// Internal utilities required to implement `Follow`.
///
/// Separated from that trait to simplify the scope of functions that are made available
/// when importing `Follow`.
pub trait FollowInternal {
    fn get_next_level_branch(
        &mut self,
        stack_index: usize,
        stack: &Stack,
    ) -> Result<Option<&mut Branch>, InternalError> {
        if stack_index < stack.len() - 1 {
            self.get_branches_at_stack_index(stack_index, stack)
                .and_then(|branches| {
                    let branch_index = stack.get(stack_index + 1).ok_or(
                        IncorrectNodeStackError::MissingBranchIndex {
                            stack_index,
                            stack: stack.clone(),
                        },
                    )?;

                    Ok((branch_index, branches))
                })
                .and_then(|(branch_index, branches)| {
                    let num_items = branches.len();

                    Some(
                        branches.get_mut(*branch_index).ok_or(
                            IncorrectNodeStackError::OutOfBounds {
                                stack_index: stack_index + 1,
                                stack: stack.clone(),
                                num_items,
                            }
                            .into(),
                        ),
                    )
                    .transpose()
                })
        } else {
            Ok(None)
        }
    }

    fn get_selected_branch(
        &mut self,
        branch_index: usize,
        stack_index: usize,
        stack: &Stack,
    ) -> Result<&mut Branch, InternalError> {
        self.get_branches_at_stack_index(stack_index, stack)
            .map_err(|err| err.into())
            .and_then(|branches| {
                let branch_choices = get_choices_from_branching_set(branches);

                branches
                    .get_mut(branch_index)
                    .ok_or(InternalError::IncorrectChoiceIndex {
                        selection: branch_index,
                        available_choices: branch_choices,
                        stack_index,
                        stack: stack.clone(),
                    })
            })
    }

    fn get_branches_at_stack_index(
        &mut self,
        stack_index: usize,
        stack: &Stack,
    ) -> Result<&mut Vec<Branch>, InternalError> {
        let num_items = self.get_num_items();

        stack
            .get(stack_index)
            .and_then(move |i| self.get_item_mut(*i))
            .ok_or(
                IncorrectNodeStackError::OutOfBounds {
                    stack_index,
                    stack: stack.clone(),
                    num_items,
                }
                .into(),
            )
            .and_then(|item| match item {
                NodeItem::BranchingPoint(branches) => Ok(branches),
                NodeItem::Line(..) => Err(IncorrectNodeStackError::ExpectedBranchingPoint {
                    stack_index,
                    stack: stack.clone(),
                }
                .into()),
            })
    }

    fn get_item(&self, index: usize) -> Option<&NodeItem>;
    fn get_item_mut(&mut self, index: usize) -> Option<&mut NodeItem>;
    fn get_num_items(&self) -> usize;
    fn get_num_visited(&self) -> u32;
    fn increment_num_visited(&mut self);
    fn iter_mut_items(&mut self) -> IterMut<NodeItem>;
}

impl FollowInternal for RootNode {
    fn get_item(&self, index: usize) -> Option<&NodeItem> {
        self.items.get(index)
    }

    fn get_item_mut(&mut self, index: usize) -> Option<&mut NodeItem> {
        self.items.get_mut(index)
    }

    fn get_num_items(&self) -> usize {
        self.items.len()
    }

    fn get_num_visited(&self) -> u32 {
        self.num_visited
    }

    fn increment_num_visited(&mut self) {
        self.num_visited += 1;
    }

    fn iter_mut_items(&mut self) -> IterMut<NodeItem> {
        self.items.iter_mut()
    }
}

impl FollowInternal for Branch {
    fn get_item(&self, index: usize) -> Option<&NodeItem> {
        self.items.get(index)
    }

    fn get_item_mut(&mut self, index: usize) -> Option<&mut NodeItem> {
        self.items.get_mut(index)
    }

    fn get_num_items(&self) -> usize {
        self.items.len()
    }

    fn get_num_visited(&self) -> u32 {
        self.num_visited
    }

    fn increment_num_visited(&mut self) {
        self.num_visited += 1;
    }

    fn iter_mut_items(&mut self) -> IterMut<NodeItem> {
        self.items.iter_mut()
    }
}

/// Collect the `ChoiceInfo` from a given set of branches.
fn get_choices_from_branching_set(branches: &[Branch]) -> Vec<ChoiceInfo> {
    branches
        .iter()
        .map(|branch| ChoiceInfo::from_choice(&branch.choice, branch.num_visited))
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        error::InklingError,
        line::{InternalChoice, LineChunkBuilder},
        node::builders::{BranchBuilder, BranchingPointBuilder, RootNodeBuilder},
        story::Address,
    };

    #[test]
    fn stack_that_points_to_line_instead_of_branching_choice_returns_error() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        match node.follow_with_choice(0, 0, &mut stack, &mut buffer) {
            Err(InklingError::Internal(InternalError::IncorrectNodeStack(err))) => match err {
                IncorrectNodeStackError::ExpectedBranchingPoint { .. } => (),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn out_of_bounds_stack_indices_return_stack_error() {
        let mut node = RootNodeBuilder::new().build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        match node.follow_with_choice(0, 0, &mut stack, &mut buffer) {
            Err(InklingError::Internal(InternalError::IncorrectNodeStack(err))) => match err {
                IncorrectNodeStackError::OutOfBounds { .. } => (),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn out_of_bounds_stack_indices_return_stack_error_when_checking_branches() {
        let mut node = RootNodeBuilder::new()
            .with_branching_choice(BranchingPointBuilder::new().build())
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0, 0, 0];

        match node.follow_with_choice(0, 0, &mut stack, &mut buffer) {
            Err(InklingError::Internal(InternalError::IncorrectNodeStack(err))) => match err {
                IncorrectNodeStackError::OutOfBounds { .. } => (),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn branch_choices_are_collected_when_supplying_an_incorrect_index_for_a_choice() {
        let internal_choice = InternalChoice::from_string("Choice");

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingPointBuilder::new()
                    .with_branch(BranchBuilder::from_choice(internal_choice.clone()).build())
                    .build(),
            )
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        match node.follow_with_choice(1, 0, &mut stack, &mut buffer) {
            Err(InklingError::Internal(InternalError::IncorrectChoiceIndex {
                selection,
                available_choices,
                ..
            })) => {
                assert_eq!(selection, 1);
                assert_eq!(available_choices.len(), 1);
                assert_eq!(available_choices[0].choice_data, internal_choice);
            }
            other => panic!("expected `InklingError::InvalidChoice` but got {:?}", other),
        }
    }

    #[test]
    fn following_items_in_a_node_adds_lines_to_buffer() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .with_text_line_chunk("Line 2")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        assert_eq!(
            node.follow(&mut stack, &mut buffer).unwrap(),
            EncounteredEvent::Done
        );

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, "Line 1");
        assert_eq!(&buffer[1].text, "Line 2");
    }

    #[test]
    fn following_into_a_node_increments_number_of_visits() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .build();

        let mut buffer = Vec::new();

        assert_eq!(node.num_visited, 0);

        node.follow(&mut vec![0], &mut buffer).unwrap();
        node.follow(&mut vec![0], &mut buffer).unwrap();

        assert_eq!(node.num_visited, 2);
    }

    #[test]
    fn following_items_updates_stack() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .with_text_line_chunk("Line 2")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow(&mut stack, &mut buffer).unwrap();
        assert_eq!(stack[0], 2);
    }

    #[test]
    fn following_items_starts_from_stack() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .with_text_line_chunk("Line 2")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![1];

        node.follow(&mut stack, &mut buffer).unwrap();

        assert_eq!(&buffer[0].text, "Line 2");
        assert_eq!(stack[0], 2);
    }

    #[test]
    fn follow_always_uses_last_position_in_stack() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .with_text_line_chunk("Line 2")
            .with_text_line_chunk("Line 3")
            .build();

        let mut buffer = Vec::new();

        let mut stack = vec![0, 2, 1];

        node.follow(&mut stack, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, "Line 2");
        assert_eq!(&buffer[1].text, "Line 3");
    }

    #[test]
    fn following_into_a_node_does_not_increment_number_of_visits_if_stack_is_non_zero() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .with_text_line_chunk("Line 2")
            .build();

        let mut buffer = Vec::new();

        assert_eq!(node.num_visited, 0);

        node.follow(&mut vec![1], &mut buffer).unwrap();

        assert_eq!(node.num_visited, 0);
    }

    #[test]
    fn following_into_line_with_divert_immediately_returns_it() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .with_line_chunk(
                LineChunkBuilder::new()
                    .with_text("Divert")
                    .with_divert("divert")
                    .build(),
            )
            .with_text_line_chunk("Line 2")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        assert_eq!(
            node.follow(&mut stack, &mut buffer).unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, "Line 1");
        assert_eq!(buffer[1].text.trim(), "Divert");
    }

    #[test]
    fn encountering_a_branching_choice_returns_the_choice_data() {
        let choice1 = InternalChoice::from_string("Choice 1");
        let choice2 = InternalChoice::from_string("Choice 2");

        let branching_choice_set = BranchingPointBuilder::new()
            .with_branch(BranchBuilder::from_choice(choice1.clone()).build())
            .with_branch(BranchBuilder::from_choice(choice2.clone()).build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(branching_choice_set)
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        match node.follow(&mut stack, &mut buffer).unwrap() {
            EncounteredEvent::BranchingChoice(choice_set) => {
                assert_eq!(choice_set.len(), 2);
                assert_eq!(choice_set[0].choice_data, choice1);
                assert_eq!(choice_set[1].choice_data, choice2);
            }
            other => panic!(
                "expected a `EncounteredEvent::BranchingChoice` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn encountering_a_branching_choice_keeps_stack_at_that_index() {
        let choice1 = InternalChoice::from_string("Choice 1");
        let choice2 = InternalChoice::from_string("Choice 2");

        let branching_choice_set = BranchingPointBuilder::new()
            .with_branch(BranchBuilder::from_choice(choice1.clone()).build())
            .with_branch(BranchBuilder::from_choice(choice2.clone()).build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .with_branching_choice(branching_choice_set)
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow(&mut stack, &mut buffer).unwrap();

        assert_eq!(stack[0], 1);
    }

    #[test]
    fn following_with_choice_follows_from_last_position_in_stack() {
        let choice = InternalChoice::from_string("Choice");
        let empty_choice = InternalChoice::from_string("");

        let empty_branch = BranchBuilder::from_choice(empty_choice.clone()).build();

        let nested_branching_choice = BranchingPointBuilder::new()
            .with_branch(empty_branch.clone())
            .with_branch(
                BranchBuilder::from_choice(choice.clone()) // Stack: [1, 2, 2], Choice: 1
                    .with_text_line_chunk("Line 3")
                    .with_text_line_chunk("Line 4")
                    .build(),
            )
            .with_branch(empty_branch.clone())
            .build();

        let nested_branch = BranchBuilder::from_choice(choice.clone())
            .with_text_line_chunk("Line 2")
            .with_branching_choice(nested_branching_choice) // Stack: [1, 2, 1]
            .build();

        let root_branching_choice = BranchingPointBuilder::new()
            .with_branch(empty_branch.clone())
            .with_branch(empty_branch.clone())
            .with_branch(nested_branch) // Stack: [1, 2]
            .build();

        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .with_branching_choice(root_branching_choice) // Stack: [1]
            .with_text_line_chunk("Line 5")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![1, 2, 2];

        node.follow_with_choice(1, 0, &mut stack, &mut buffer)
            .unwrap();

        assert_eq!(&buffer[1].text, "Line 3");
        assert_eq!(&buffer[2].text, "Line 4");
    }

    #[test]
    fn after_finishing_with_a_branch_lower_nodes_return_to_their_content() {
        let choice = InternalChoice::from_string("Choice");

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingPointBuilder::new()
                    .with_branch(BranchBuilder::from_choice(choice).build())
                    .build(),
            )
            .with_text_line_chunk("Line 1")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap();

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[1].text, "Line 1");

        assert_eq!(&stack, &[2]);
    }

    #[test]
    fn selected_branches_have_their_number_of_visits_number_incremented() {
        let choice = InternalChoice::from_string("Choice");

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingPointBuilder::new()
                    .with_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .with_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .with_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .build(),
            )
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow_with_choice(1, 0, &mut stack, &mut buffer)
            .unwrap();

        match &node.items[0] {
            NodeItem::BranchingPoint(branches) => {
                assert_eq!(branches[0].num_visited, 0);
                assert_eq!(branches[1].num_visited, 1);
                assert_eq!(branches[2].num_visited, 0);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn encountered_choices_return_with_their_number_of_visits_counter() {
        let choice = InternalChoice::from_string("Choice");

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingPointBuilder::new()
                    .with_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .build(),
            )
            .build();

        let mut buffer = Vec::new();

        node.follow_with_choice(0, 0, &mut vec![0], &mut buffer)
            .unwrap();
        node.follow_with_choice(0, 0, &mut vec![0], &mut buffer)
            .unwrap();
        node.follow_with_choice(0, 0, &mut vec![0], &mut buffer)
            .unwrap();

        match node.follow(&mut vec![0], &mut buffer).unwrap() {
            EncounteredEvent::BranchingChoice(branches) => {
                assert_eq!(branches[0].num_visited, 3);
            }
            other => panic!(
                "expected a `EncounteredEvent::BranchingChoice` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn selected_branches_adds_line_text_to_line_buffer() {
        let choice = InternalChoice::from_string("Choice");

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingPointBuilder::new()
                    .with_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .build(),
            )
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap();

        assert_eq!(&buffer[0].text, "Choice");
    }

    #[test]
    fn diverts_found_after_selections_are_returned() {
        let choice = InternalChoice::from_string("Choice -> divert");

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingPointBuilder::new()
                    .with_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .build(),
            )
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        assert_eq!(
            node.follow_with_choice(0, 0, &mut stack, &mut buffer)
                .unwrap(),
            EncounteredEvent::Divert(Address::Raw("divert".to_string()))
        );
    }

    #[test]
    fn following_into_nested_branches_works() {
        let choice = InternalChoice::from_string("Choice");

        let nested_branch = BranchingPointBuilder::new()
            .with_branch(BranchBuilder::from_choice(choice.clone()).build())
            .build();

        let branch_set = BranchingPointBuilder::new()
            .with_branch(
                BranchBuilder::from_choice(choice.clone())
                    .with_branching_choice(nested_branch)
                    .build(),
            )
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(branch_set)
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        match node
            .follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap()
        {
            EncounteredEvent::BranchingChoice(branches) => assert_eq!(branches.len(), 1),
            other => panic!("expected a `BranchingChoice` but got {:?}", other),
        }
    }

    #[test]
    fn after_a_followed_choice_returns_the_caller_nodes_always_follow_into_their_next_lines() {
        let choice = InternalChoice::from_string("Choice");

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingPointBuilder::new()
                    .with_branch(
                        BranchBuilder::from_choice(choice.clone())
                            .with_branching_choice(
                                BranchingPointBuilder::new()
                                    .with_branch(
                                        BranchBuilder::from_choice(choice.clone())
                                            .with_branching_choice(
                                                BranchingPointBuilder::new()
                                                    .with_branch(
                                                        BranchBuilder::from_choice(choice.clone())
                                                            .with_text_line_chunk("Line 1")
                                                            .build(),
                                                    )
                                                    .build(),
                                            )
                                            .with_text_line_chunk("Line 2")
                                            .build(),
                                    )
                                    .build(),
                            )
                            .with_text_line_chunk("Line 3")
                            .build(),
                    )
                    .build(),
            )
            .with_text_line_chunk("Line 4")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap();
        node.follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap();
        node.follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap();

        assert_eq!(buffer.len(), 7);
        assert_eq!(&buffer[3].text, "Line 1");
        assert_eq!(&buffer[4].text, "Line 2");
        assert_eq!(&buffer[5].text, "Line 3");
        assert_eq!(&buffer[6].text, "Line 4");
    }

    #[test]
    fn following_with_stack_that_has_too_large_index_raises_error() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .build();

        let mut buffer = Vec::new();

        match node.follow(&mut vec![2], &mut buffer) {
            Err(InklingError::Internal(InternalError::IncorrectNodeStack(err))) => match err {
                IncorrectNodeStackError::OutOfBounds { .. } => (),
                err => panic!(
                    "expected `IncorrectNodeStackError::OutOfBounds` but got {:?}",
                    err
                ),
            },
            err => panic!(
                "expected `IncorrectNodeStackError::OutOfBounds` but got {:?}",
                err
            ),
        }
    }

    #[test]
    fn following_with_empty_stack_raises_error() {
        let mut node = RootNodeBuilder::new()
            .with_text_line_chunk("Line 1")
            .build();

        let mut buffer = Vec::new();

        match node.follow(&mut vec![], &mut buffer) {
            Err(InklingError::Internal(InternalError::IncorrectNodeStack(err))) => match err {
                IncorrectNodeStackError::EmptyStack => (),
                err => panic!(
                    "expected `IncorrectNodeStackError::EmptyStack` but got {:?}",
                    err
                ),
            },
            err => panic!(
                "expected `IncorrectNodeStackError::EmptyStack` but got {:?}",
                err
            ),
        }
    }
}
