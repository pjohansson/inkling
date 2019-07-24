use crate::{
    error::{InklingError, InternalError},
    follow::{FollowResult, LineDataBuffer, Next},
    line::{ChoiceData, Process},
    node::{Branch, Container, RootNode},
};

/// Represents the current stack of choices that have been made from the root
/// of the current graph (in a practical sense, that have been made inside the
/// current `Stitch`).
///
/// For example, for this tree:
///
/// Root
/// ===
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
/// ===
///
/// the current stack is [2, 1, 1]. When the user picks a choice the stack is used to
/// advance to the position of that choice set in the tree, then follow from there on.
///
/// Do note that every `Branch` adds a line of text to its children. Lines after this
/// choice start at index 1.
pub type Stack = Vec<usize>;

pub trait Follow {
    fn follow(&mut self, stack: &mut Stack, buffer: &mut LineDataBuffer) -> FollowResult {
        let at_index = stack.last_mut().ok_or(String::new())?;

        if *at_index == 0 {
            self.increment_num_visited();
        }

        for item in self.items().get_mut(*at_index..).unwrap().iter_mut() {
            *at_index += 1;

            match item {
                Container::Line(line) => {
                    let result = line.process(buffer)?;

                    if let Next::Divert(..) = result {
                        return Ok(result);
                    }
                }
                Container::BranchingChoice(branches) => {
                    *at_index -= 1;

                    let branching_choice_set = get_choices_from_branching_set(branches);

                    return Ok(Next::ChoiceSet(branching_choice_set));
                }
            }
        }

        Ok(Next::Done)
    }

    fn follow_with_choice(
        &mut self,
        chosen_branch_index: usize,
        stack_index: usize,
        stack: &mut Stack,
        buffer: &mut LineDataBuffer,
    ) -> FollowResult
    where
        Self: Sized,
    {
        let result = if stack_index < stack.len() - 1 {
            let next_branch = get_next_level_branch(stack_index, stack, self)?;
            next_branch.follow_with_choice(chosen_branch_index, stack_index + 2, stack, buffer)
        } else {
            let selected_branch =
                match get_selected_branch(chosen_branch_index, stack_index, stack, self) {
                    Ok(branch) => branch,
                    Err(err) => {
                        let err = if let Some(user_err) =
                            check_for_invalid_choice(chosen_branch_index, stack_index, stack, self)
                        {
                            user_err
                        } else {
                            err.into()
                        };

                        return Err(err);
                    }
                };

            stack.extend_from_slice(&[chosen_branch_index, 0]);

            selected_branch.follow(stack, buffer)
        }?;

        match result {
            Next::Done => {
                stack.truncate(stack.len() - 2);
                *stack.last_mut().expect("stack.last_mut") += 1;

                self.follow(stack, buffer)
            }
            other => Ok(other),
        }
    }

    fn get_item(&self, index: usize) -> Option<&Container>;
    fn get_item_mut(&mut self, index: usize) -> Option<&mut Container>;
    fn get_num_items(&self) -> usize;
    fn get_num_visited(&self) -> u32;
    fn increment_num_visited(&mut self);
    fn items(&mut self) -> Vec<&mut Container>;
}

impl Follow for RootNode {
    fn get_item(&self, index: usize) -> Option<&Container> {
        self.items.get(index)
    }

    fn get_item_mut(&mut self, index: usize) -> Option<&mut Container> {
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

    fn items(&mut self) -> Vec<&mut Container> {
        self.items.iter_mut().collect()
    }
}

impl Follow for Branch {
    fn get_item(&self, index: usize) -> Option<&Container> {
        self.items.get(index)
    }

    fn get_item_mut(&mut self, index: usize) -> Option<&mut Container> {
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

    fn items(&mut self) -> Vec<&mut Container> {
        self.items.iter_mut().collect()
    }
}

fn check_for_invalid_choice<T: Follow>(
    chosen_branch_index: usize,
    stack_index: usize,
    stack: &Stack,
    node: &T,
) -> Option<InklingError> {
    let branch_set_index = stack.get(stack_index)?;

    if let Some(Container::BranchingChoice(branches)) = &node.get_item(*branch_set_index) {
        if chosen_branch_index >= branches.len() {
            return Some(get_invalid_choice_error_stub(branches, chosen_branch_index));
        }
    }

    None
}

fn get_selected_branch<'a, T: Follow + Sized>(
    chosen_branch_index: usize,
    stack_index: usize,
    stack: &Stack,
    node: &'a mut T,
) -> Result<&'a mut Branch, InternalError> {
    let branch_set_index = stack[stack_index];
    let num_items = node.get_num_items();

    let item = node
        .get_item_mut(branch_set_index)
        .ok_or(InternalError::bad_indices(
            stack_index,
            branch_set_index,
            num_items,
            stack,
        ))?;

    match item {
        Container::BranchingChoice(branches) => {
            branches
                .get_mut(chosen_branch_index)
                .ok_or(InternalError::bad_indices(
                    stack_index + 1,
                    chosen_branch_index,
                    num_items,
                    &stack,
                ))
        }
        err => {
            unimplemented!();
        }
    }
}

fn get_choices_from_branching_set(branches: &[Branch]) -> Vec<ChoiceData> {
    branches
        .iter()
        .map(|branch| {
            let num_visited = branch.num_visited;

            ChoiceData {
                num_visited,
                ..branch.choice.clone()
            }
        })
        .collect::<Vec<_>>()
}

fn get_next_level_branch<'a, T: Follow + Sized>(
    stack_index: usize,
    stack: &Stack,
    node: &'a mut T,
) -> Result<&'a mut Branch, InternalError> {
    let branch_set_index = stack[stack_index];
    let branch_index = stack[stack_index + 1];
    let num_items = node.get_num_items();

    let item = node
        .get_item_mut(branch_set_index)
        .ok_or(InternalError::bad_indices(
            stack_index,
            branch_set_index,
            num_items,
            stack,
        ))?;

    match item {
        Container::BranchingChoice(branches) => {
            branches
                .get_mut(branch_index)
                .ok_or(InternalError::bad_indices(
                    stack_index + 1,
                    branch_index,
                    num_items,
                    stack,
                ))
        }
        _ => {
            unimplemented!();
        }
    }
}

/// If the used index to select a choice with was wrong, construct a stub of the error
/// with type `InklingError::InvalidChoice`. Here we fill in which index caused
/// the error and the full list of available choices that it tried to select from.
///
/// The other fields should be filled in by later error handling if needed: this is
/// the information that the current node has direct access to.
fn get_invalid_choice_error_stub(
    branching_choice_set: &[Branch],
    choice_index: usize,
) -> InklingError {
    let choices = get_choices_from_branching_set(branching_choice_set);

    InklingError::InvalidChoice {
        index: choice_index,
        choice: None,
        presented_choices: Vec::new(),
        internal_choices: choices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::node::tests::BranchingChoiceBuilder;
    use super::super::node::*;

    use crate::line::{
        choice::tests::ChoiceBuilder as ChoiceDataBuilder,
        line::tests::LineBuilder as LineDataBuilder,
    };

    // #[test]
    // fn follow_with_user_choice_not_in_the_set_returns_invalid_choice_error() {
    //     let choice = 2;
    //     let choice_set = get_choice_set_with_empty_choices(choice);

    //     let node = DialogueNode::with_items(vec![choice_set]);

    //     let mut buffer = Vec::new();
    //     let mut stack = vec![0];

    //     match node.follow_with_choice(choice, 0, &mut buffer, &mut stack) {
    //         Err(InklingError::InvalidChoice { .. }) => (),
    //         _ => panic!("`InklingError::InvalidChoice` was not yielded"),
    //     }
    // }

    // #[test]
    // fn choice_and_index_collection_when_picking_a_choice_with_a_bad_index() {
    //     let line1 = LineData::from_str("choice 1").unwrap();
    //     let line2 = LineData::from_str("choice 2").unwrap();

    //     let choice_set_items = vec![
    //         NodeItem::Node {
    //             kind: NodeType::Choice(ChoiceBuilder::empty().with_line(line1).build()),
    //             node: Box::new(DialogueNode::with_items(vec![])),
    //         },
    //         NodeItem::Node {
    //             kind: NodeType::Choice(ChoiceBuilder::empty().with_line(line2).build()),
    //             node: Box::new(DialogueNode::with_items(vec![])),
    //         },
    //     ];

    //     let node = DialogueNode::with_items(choice_set_items);

    //     let choice_set = NodeItem::Node {
    //         kind: NodeType::ChoiceSet,
    //         node: Box::new(node),
    //     };

    //     let error = get_invalid_choice_error_stub(&choice_set, 2);

    //     match error {
    //         InklingError::InvalidChoice {
    //             index,
    //             choice,
    //             presented_choices,
    //             internal_choices,
    //         } => {
    //             assert_eq!(index, 2);
    //             assert_eq!(internal_choices.len(), 2);
    //             assert_eq!(&internal_choices[0].line.text, "choice 1");
    //             assert_eq!(&internal_choices[1].line.text, "choice 2");

    //             // Not filled in yet
    //             assert!(choice.is_none());
    //             assert!(presented_choices.is_empty());
    //         }
    //         _ => panic!(
    //             "expected an `InklingError::InvalidChoice` object but got {:?}",
    //             error
    //         ),
    //     }
    // }

    #[test]
    fn following_items_in_a_node_adds_lines_to_buffer() {
        let mut node = RootNodeBuilder::new()
            .with_line_text("Line 1")
            .with_line_text("Line 2")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        assert_eq!(node.follow(&mut stack, &mut buffer).unwrap(), Next::Done);

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, "Line 1");
        assert_eq!(&buffer[1].text, "Line 2");
    }

    #[test]
    fn following_into_a_node_increments_number_of_visits() {
        let mut node = RootNodeBuilder::new().with_line_text("Line 1").build();

        let mut buffer = Vec::new();

        assert_eq!(node.num_visited, 0);

        node.follow(&mut vec![0], &mut buffer).unwrap();
        node.follow(&mut vec![0], &mut buffer).unwrap();

        assert_eq!(node.num_visited, 2);
    }

    #[test]
    fn following_items_updates_stack() {
        let mut node = RootNodeBuilder::new()
            .with_line_text("Line 1")
            .with_line_text("Line 2")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow(&mut stack, &mut buffer).unwrap();
        assert_eq!(stack[0], 2);
    }

    #[test]
    fn following_items_starts_from_stack() {
        let mut node = RootNodeBuilder::new()
            .with_line_text("Line 1")
            .with_line_text("Line 2")
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
            .with_line_text("Line 1")
            .with_line_text("Line 2")
            .with_line_text("Line 3")
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
            .with_line_text("Line 1")
            .with_line_text("Line 2")
            .build();

        let mut buffer = Vec::new();

        assert_eq!(node.num_visited, 0);

        node.follow(&mut vec![1], &mut buffer).unwrap();

        assert_eq!(node.num_visited, 0);
    }

    #[test]
    fn following_into_line_with_divert_immediately_returns_it() {
        let mut node = RootNodeBuilder::new()
            .with_line_text("Line 1")
            .with_line_text("Divert -> divert")
            .with_line_text("Line 2")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        assert_eq!(
            node.follow(&mut stack, &mut buffer).unwrap(),
            Next::Divert("divert".to_string())
        );

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, "Line 1");
        assert_eq!(buffer[1].text.trim(), "Divert");
    }

    #[test]
    fn encountering_a_branching_choice_returns_the_choice_data() {
        let choice1 = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("Choice 1").build())
            .build();
        let choice2 = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("Choice 2").build())
            .build();

        let branching_choice_set = BranchingChoiceBuilder::new()
            .add_branch(BranchBuilder::from_choice(choice1.clone()).build())
            .add_branch(BranchBuilder::from_choice(choice2.clone()).build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(branching_choice_set)
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        match node.follow(&mut stack, &mut buffer).unwrap() {
            Next::ChoiceSet(choice_set) => {
                assert_eq!(choice_set.len(), 2);
                assert_eq!(choice_set[0], choice1);
                assert_eq!(choice_set[1], choice2);
            }
            other => panic!("expected a `Next::ChoiceSet` but got {:?}", other),
        }
    }

    #[test]
    fn encountering_a_branching_choice_keeps_stack_at_that_index() {
        let choice1 = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("Choice 1").build())
            .build();
        let choice2 = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("Choice 2").build())
            .build();

        let branching_choice_set = BranchingChoiceBuilder::new()
            .add_branch(BranchBuilder::from_choice(choice1.clone()).build())
            .add_branch(BranchBuilder::from_choice(choice2.clone()).build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_line_text("Line 1")
            .with_branching_choice(branching_choice_set)
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow(&mut stack, &mut buffer).unwrap();

        assert_eq!(stack[0], 1);
    }

    #[test]
    fn following_with_choice_follows_from_last_position_in_stack() {
        let choice = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("Choice").build())
            .build();

        let empty_choice = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("").build())
            .build();

        let empty_branch = BranchBuilder::from_choice(empty_choice.clone()).build();

        let nested_branching_choice = BranchingChoiceBuilder::new()
            .add_branch(empty_branch.clone())
            .add_branch(
                BranchBuilder::from_choice(choice.clone()) // Stack: [1, 2, 2], Choice: 1
                    .with_line_text("Line 3")
                    .with_line_text("Line 4")
                    .build(),
            )
            .add_branch(empty_branch.clone())
            .build();

        let nested_branch = BranchBuilder::from_choice(choice.clone())
            .with_line_text("Line 2")
            .with_branching_choice(nested_branching_choice) // Stack: [1, 2, 1]
            .build();

        let root_branching_choice = BranchingChoiceBuilder::new()
            .add_branch(empty_branch.clone())
            .add_branch(empty_branch.clone())
            .add_branch(nested_branch) // Stack: [1, 2]
            .build();

        let mut node = RootNodeBuilder::new()
            .with_line_text("Line 1")
            .with_branching_choice(root_branching_choice) // Stack: [1]
            .with_line_text("Line 5")
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
        let choice = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("Choice").build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingChoiceBuilder::new()
                    .add_branch(BranchBuilder::from_choice(choice).build())
                    .build(),
            )
            .with_line_text("Line 1")
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
        let choice = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("Choice").build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingChoiceBuilder::new()
                    .add_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .add_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .add_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .build(),
            )
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow_with_choice(1, 0, &mut stack, &mut buffer)
            .unwrap();

        match &node.items[0] {
            Container::BranchingChoice(branches) => {
                assert_eq!(branches[0].num_visited, 0);
                assert_eq!(branches[1].num_visited, 1);
                assert_eq!(branches[2].num_visited, 0);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn encountered_choices_return_with_their_number_of_visits_counter() {
        let choice = ChoiceDataBuilder::empty()
            .with_displayed(LineDataBuilder::new("Choice").build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingChoiceBuilder::new()
                    .add_branch(BranchBuilder::from_choice(choice.clone()).build())
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
            Next::ChoiceSet(branches) => {
                assert_eq!(branches[0].num_visited, 3);
            }
            other => panic!("expected a `Next::ChoiceSet` but got {:?}", other),
        }
    }

    #[test]
    fn selected_branches_adds_line_text_to_line_buffer() {
        let choice = ChoiceDataBuilder::empty()
            .with_line(LineDataBuilder::new("Choice").build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingChoiceBuilder::new()
                    .add_branch(BranchBuilder::from_choice(choice.clone()).build())
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
        let choice = ChoiceDataBuilder::empty()
            .with_line(LineDataBuilder::new("Choice").with_divert("divert").build())
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingChoiceBuilder::new()
                    .add_branch(BranchBuilder::from_choice(choice.clone()).build())
                    .build(),
            )
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        assert_eq!(
            node.follow_with_choice(0, 0, &mut stack, &mut buffer)
                .unwrap(),
            Next::Divert("divert".to_string())
        );
    }

    #[test]
    fn following_into_nested_branches_works() {
        let choice = ChoiceDataBuilder::empty()
            .with_line(LineDataBuilder::new("Choice").build())
            .build();

        let nested_branch = BranchingChoiceBuilder::new()
            .add_branch(BranchBuilder::from_choice(choice.clone()).build())
            .build();

        let branch_set = BranchingChoiceBuilder::new()
            .add_branch(
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
            Next::ChoiceSet(branches) => assert_eq!(branches.len(), 1),
            other => panic!("expected a `ChoiceSet` but got {:?}", other),
        }
    }

    #[test]
    fn after_a_followed_choice_returns_the_caller_nodes_always_follow_into_their_next_lines() {
        let choice = ChoiceDataBuilder::empty()
            .with_line(LineDataBuilder::new("Choice").build())
            .build();

        let nested_branch = BranchingChoiceBuilder::new()
            .add_branch(
                BranchBuilder::from_choice(choice.clone())
                    .with_line_text("Line 1")
                    .build(),
            )
            .build();

        let branch_set = BranchingChoiceBuilder::new()
            .add_branch(
                BranchBuilder::from_choice(choice.clone())
                    .with_branching_choice(nested_branch)
                    .with_line_text("Line 2")
                    .build(),
            )
            .build();

        let mut node = RootNodeBuilder::new()
            .with_branching_choice(
                BranchingChoiceBuilder::new()
                    .add_branch(
                        BranchBuilder::from_choice(choice.clone())
                            .with_branching_choice(
                                BranchingChoiceBuilder::new()
                                    .add_branch(
                                        BranchBuilder::from_choice(choice.clone())
                                            .with_branching_choice(
                                                BranchingChoiceBuilder::new()
                                                    .add_branch(
                                                        BranchBuilder::from_choice(choice.clone())
                                                            .with_line_text("Line 1")
                                                            .build(),
                                                    )
                                                    .build(),
                                            )
                                            .with_line_text("Line 2")
                                            .build(),
                                    )
                                    .build(),
                            )
                            .with_line_text("Line 3")
                            .build(),
                    )
                    .build(),
            )
            .with_line_text("Line 4")
            .build();

        let mut buffer = Vec::new();
        let mut stack = vec![0];

        node.follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap();
        node.follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap();
        node.follow_with_choice(0, 0, &mut stack, &mut buffer)
            .unwrap();

        dbg!(&buffer);

        assert_eq!(buffer.len(), 7);
        assert_eq!(&buffer[3].text, "Line 1");
        assert_eq!(&buffer[4].text, "Line 2");
        assert_eq!(&buffer[5].text, "Line 3");
        assert_eq!(&buffer[6].text, "Line 4");
    }

    #[test]
    fn following_with_empty_stack_raises_error() {
        let mut node = RootNodeBuilder::new().with_line_text("Line 1").build();

        let mut buffer = Vec::new();

        assert!(node.follow(&mut vec![], &mut buffer).is_err());
    }
}
