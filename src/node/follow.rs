use crate::{
    knot::{LineBuffer, Next},
    line::{Choice, LineKind},
};

use super::node::{DialogueNode, NodeItem, NodeType};

impl DialogueNode {
    pub fn follow(
        &self,
        current_level: usize,
        buffer: &mut LineBuffer,
        stack: &mut Vec<usize>,
    ) -> Result<Next, String> {
        let index = get_stack_index_for_current_level(current_level, stack)?;

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
                    let choices = get_choices_from_set(item)?;

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

    pub fn follow_with_choice(
        &self,
        choice: usize,
        buffer: &mut LineBuffer,
        stack: &mut Vec<usize>,
    ) -> Result<Next, String> {
        let index = stack.last().unwrap();
        let item = &self.items[*index];

        let choice_node = get_choice_set_node(item)?
            .items
            .get(choice)
            .ok_or(format!(
                "selected choice with index {} does not exist in set",
                choice
            ))
            .and_then(|item| get_choice_node(item))?;

        stack.push(choice);

        let result = choice_node.follow(2, buffer, stack)?;

        match &result {
            Next::Done => {
                // Continue reading the current node's items
                stack.truncate(stack.len() - 2);
                stack[0] += 1;
                self.follow(0, buffer, stack)
            }
            _ => Ok(result),
        }
    }
}

fn get_choice_set_node(item: &NodeItem) -> Result<&DialogueNode, String> {
    match &item {
        NodeItem::Node {
            kind: NodeType::ChoiceSet,
            node,
        } => Ok(node),
        _ => Err(format!(
            "expected to find a `ChoiceSet` node, instead found {:?}",
            item
        )),
    }
}

fn get_choice_node(item: &NodeItem) -> Result<&DialogueNode, String> {
    match &item {
        NodeItem::Node {
            kind: NodeType::Choice(..),
            node,
        } => Ok(node),
        _ => Err(format!(
            "expected to find a `Choice` node, instead found {:?}",
            item
        )),
    }
}

// This should only ever be called on a `NodeItem` which is a `ChoiceSet`. Since we
// are calling this internally this should only ever be the case.
fn get_choices_from_set(choice_set: &NodeItem) -> Result<Vec<Choice>, String> {
    match choice_set {
        NodeItem::Node {
            kind: NodeType::ChoiceSet,
            node,
        } => node
            .items
            .iter()
            .map(|item| get_choice_from_node_item(item))
            .collect(),
        _ => {
            return Err(format!(
                "tried to collect a set of choices from a non-`ChoiceSet` node (was: {:?})",
                &choice_set
            ));
        }
    }
}

// This should only ever be called on a `NodeItem` which is a `Choice`. Since we
// are calling this internally this should only ever be the case.
fn get_choice_from_node_item(item: &NodeItem) -> Result<Choice, String> {
    match item {
        NodeItem::Node {
            kind: NodeType::Choice(choice),
            ..
        } => Ok(choice.clone()),
        _ => Err(format!(
            "tried to collect a `Choice` from a non-`Choice` node (was: {:?})",
            &item
        )),
    }
}

/// Get a mutable stack index for the current level. If the current stack has no entry
/// for the current level, add and return it starting from 0. Return an error if the
/// stack is incomplete, either by not containing all previous stack entries for nodes
/// or if it has not been truncated to the current node level.
fn get_stack_index_for_current_level(
    current_level: usize,
    stack: &mut Vec<usize>,
) -> Result<&mut usize, String> {
    if stack.len() < current_level {
        return Err(
            "Cannot set current index on stack: node level is {}, but current stack \
             only has {} entries. One or more stack indices is missing in between and \
             the current node cannot make an assumption of the previous stack trace."
                .to_string(),
        );
    } else if stack.len() > current_level + 1 {
        return Err(format!(
            "Current stack ({:?}, length: {}) is not truncated to current node level {}",
            &stack,
            stack.len(),
            current_level
        ));
    }

    if stack.len() == current_level {
        stack.push(0);
    }

    Ok(stack.last_mut().unwrap())
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

        node.follow_with_choice(1, &mut buffer, &mut stack).unwrap();

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

        node.follow_with_choice(choice, &mut buffer, &mut stack)
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

        node.follow_with_choice(0, &mut buffer, &mut stack).unwrap();

        assert_eq!(&stack, &[2]);
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
