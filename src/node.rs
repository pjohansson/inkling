use crate::line::{Choice, Line, ParsedLine};

#[derive(Debug)]
/// Node in a graph representation of a dialogue tree.
pub struct DialogueNode {
    /// Children of current node.
    items: Vec<NodeItem>,
}

impl DialogueNode {
    /// Parse a set of `ParsedLine` items and return a full graph representation of it.
    pub fn from_lines(lines: &[ParsedLine]) -> Self {
        parse_full_node(lines)
    }
}

#[derive(Debug)]
enum NodeItem {
    /// Regular line of marked up text.
    Line(Line),
    /// Nested node, either a `MultiChoice` which has `Choices` as children, or a `Choice`
    /// which has more `Line`s and possibly further `MultiChoice`s.
    Node { kind: NodeType, node: Box<DialogueNode> },
}

#[derive(Debug)]
enum NodeType {
    /// Root of a set of choices. All node items will be of type `Choice`.
    MultiChoice,
    /// Choice in a set of choices. All node items will be lines or further `MultiChoice` nodes.
    Choice(Choice),
}

/// Used to parse the input lines from beginning to end, returning the constructed `Node`.
fn parse_full_node(lines: &[ParsedLine]) -> DialogueNode {
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
                let (item, gather) = parse_multichoice_with_gather(&mut index, *level, lines);
                items.push(item);

                if let Some(line) = gather {
                    items.push(line);
                    index -= 1;
                }
            }
            _ => (),
        };

        index += 1;
    }

    DialogueNode { items }
}

/// After parsing a group of choices, check whether it ended because of a `Gather`. 
/// If so, return the `NodeItem::Line` object from that gather so that it can be appended
/// *after* the node, not inside it.
fn parse_multichoice_with_gather(index: &mut usize, current_level: u8, lines: &[ParsedLine]) 
-> (NodeItem, Option<NodeItem>) {
    let node = parse_multichoice(index, current_level, lines);
    let mut gather = None;

    if let Some(ParsedLine::Gather { level, line }) = lines.get(*index) {
        if *level == current_level {
            gather.replace(NodeItem::Line(line.clone()));
            *index += 1;
        }
    }

    (node, gather)
}

/// Parse a set of `Choice`s with the same level, grouping them into a single `MultiChoice`
/// node that is returned.
fn parse_multichoice(index: &mut usize, current_level: u8, lines: &[ParsedLine]) -> NodeItem {
    let mut choices = Vec::new();

    while let Some(choice) = parse_choice(index, current_level, lines) {
        choices.push(choice);
    }

    let node = DialogueNode { items: choices };

    NodeItem::Node {
        kind: NodeType::MultiChoice,
        node: Box::new(node),
    }
}

/// Parse a single `Choice` node. The node ends either when another `Choice` node with 
/// the same level or below is encountered, when a `Gather` with the same level or below
/// is encountered or when all lines are read.
fn parse_choice(index: &mut usize, current_level: u8, lines: &[ParsedLine]) -> Option<NodeItem> {
    if *index >= lines.len() {
        return None;
    }

    let head = &lines[*index];

    let choice: Choice = match head {
        ParsedLine::Choice { level, .. } if *level < current_level => {
            return None;
        }
        ParsedLine::Gather { level, .. } if *level <= current_level => {
            return None;
        },
        ParsedLine::Choice { choice, .. } => choice.clone(),
        _ => panic!(
            "could not correctly parse a `NodeItem` of type `Choice`: \
             expected first line to be a `ParsedLine::Choice` object, but was {:?}",
            &head
        ),
    };

    // This skips to the next index, where the choice's content or a new choice will appear
    let mut items = Vec::new();
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
                let (multi_choice, gather) = parse_multichoice_with_gather(index, *level, lines);

                items.push(multi_choice);

                if let Some(line) = gather {  
                    items.push(line);
                }

                // `parse_multichoice` advances the index to the next line after the group,
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

    let node = DialogueNode { items };

    Some(NodeItem::Node {
        kind: NodeType::Choice(choice),
        node: Box::new(node),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ops::Index, str::FromStr};

    fn get_empty_choice(level: u8) -> ParsedLine {
        let choice = Choice { selection_text: String::new(), line: Line::from_str("").unwrap() };
        ParsedLine::Choice { level, choice }
    }

    fn get_empty_gather(level: u8) -> ParsedLine {
        let line = Line::from_str("").unwrap();
        ParsedLine::Gather { level, line }
    }

    fn get_line(s: &str) -> ParsedLine {
        ParsedLine::Line(Line::from_str(s).unwrap())
    }

    #[test]
    fn parsing_choices_at_same_level_returns_when_encountering_other_choice() {
        let level = 1;
        let choice = get_empty_choice(level);

        let lines = vec![choice.clone(), choice.clone(), choice.clone()];

        let mut index = 0;

        let first = parse_choice(&mut index, level, &lines).unwrap();
        assert_eq!(index, 1);
        assert!(first.is_choice());
        assert!(first.node().items.is_empty());

        let second = parse_choice(&mut index, level, &lines).unwrap();
        assert_eq!(index, 2);
        assert!(second.is_choice());
        assert!(second.node().items.is_empty());

        let third = parse_choice(&mut index, level, &lines).unwrap();
        assert_eq!(index, 3);
        assert!(third.is_choice());
        assert!(third.node().items.is_empty());

        assert!(parse_choice(&mut index, level, &lines).is_none());
    }

    #[test]
    fn parsing_choice_returns_choice_data_in_root() {
        let selection_text = "Netherfield Park".to_string();
        let text = "\"To Netherfield Park, then\", I exclaimed.";

        let line = Line::from_str(text).unwrap();
        let choice = Choice {
            selection_text,
            line,
        };

        let input = ParsedLine::Choice {
            level: 1,
            choice: choice.clone(),
        };

        let lines = vec![input];

        let mut index = 0;

        match parse_choice(&mut index, 1, &lines).unwrap() {
            NodeItem::Node {
                kind: NodeType::Choice(parsed),
                ..
            } => {
                assert_eq!(parsed, choice);
            }
            _ => panic!("result not a `NodeItem::Node { kind: NodeType::Choice, .. }"),
        }
    }

    #[test]
    fn parsing_choices_adds_found_regular_lines_as_items() {
        let level = 1;
        let choice = get_empty_choice(level);
        let line = get_line("");

        let lines = vec![
            choice.clone(),
            line.clone(),
            line.clone(),
            choice.clone(),
            line.clone(),
        ];

        let mut index = 0;

        let first = parse_choice(&mut index, level, &lines).unwrap();
        let second = parse_choice(&mut index, level, &lines).unwrap();
        assert_eq!(index, 5);

        assert!(first.is_choice());
        assert_eq!(first.len(), 2);
        assert!(first.node().items[0].is_line());
        assert!(first.node().items[1].is_line());

        assert!(second.is_choice());
        assert_eq!(second.len(), 1);
        assert!(second.node().items[0].is_line());
    }

    #[test]
    fn parsing_choices_stops_when_a_lower_level_choice_is_encountered() {
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);

        let lines = vec![choice2.clone(), choice2.clone(), choice1.clone()];

        let mut index = 0;

        assert!(parse_choice(&mut index, 2, &lines).is_some());
        assert!(parse_choice(&mut index, 2, &lines).is_some());
        assert!(parse_choice(&mut index, 2, &lines).is_none());

        assert_eq!(index, 2);
    }

    #[test]
    fn parsing_multichoice_returns_all_choices_with_nested_content() {
        let choice = get_empty_choice(1);
        let line1 = ParsedLine::Line(Line::from_str("one").unwrap());
        let line2 = ParsedLine::Line(Line::from_str("two").unwrap());
        let line3 = ParsedLine::Line(Line::from_str("three").unwrap());

        let lines = vec![
            choice.clone(),
            line1.clone(),
            line2.clone(),
            choice.clone(),
            line3.clone(),
            choice.clone(),
        ];

        let mut index = 0;

        let root = parse_multichoice(&mut index, 1, &lines);

        assert_eq!(index, 6);
        assert!(root.is_multichoice());

        assert_eq!(root.len(), 3);

        for item in &root.node().items {
            assert!(item.is_choice());
        }

        assert_eq!(root[0].len(), 2);
        assert_eq!(root[1].len(), 1);
        assert_eq!(root[2].len(), 0);
    }

    #[test]
    fn parsing_choices_turns_nested_choices_into_multichoices() {
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);
        let lines = vec![choice1.clone(), choice2.clone()];

        let mut index = 0;

        let root = parse_choice(&mut index, 1, &lines).unwrap();

        assert!(root.is_choice());
        assert_eq!(index, 2);

        assert_eq!(root.len(), 1);
        assert!(root[0].is_multichoice());
        assert!(root[0][0].is_choice());
    }

    #[test]
    fn parsing_multichoice_returns_early_if_lower_level_choice_is_encountered() {
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);

        let lines = vec![choice2.clone(), choice1.clone()];

        let mut index = 0;

        let root = parse_multichoice(&mut index, 2, &lines);

        assert_eq!(index, 1);
        assert_eq!(root.len(), 1);
    }

    #[test]
    fn parsing_multichoice_returns_handles_multiple_simultaneous_drops_in_level() {
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

        let root = parse_multichoice(&mut index, 2, &lines);

        assert_eq!(index, 4);
        assert_eq!(root.len(), 2);
    }

    #[test]
    fn parsing_complex_nested_structure_works() {
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);
        let choice3 = get_empty_choice(3);
        let choice4 = get_empty_choice(4);

        let line = get_line("");

        let lines = vec![
            choice1.clone(),
            choice2.clone(),
            choice2.clone(),
            choice1.clone(),
            choice2.clone(),
            choice3.clone(),
            line.clone(),
            line.clone(),
            choice4.clone(),
            choice1.clone(),
        ];

        let mut index = 0;

        let root = parse_multichoice(&mut index, 1, &lines);

        assert_eq!(index, 10);
        assert!(root.is_multichoice());

        // Assert that the level 3 choice has two lines and one final multichoice
        let node = &root[1][0][0][0][0];

        assert_eq!(node.len(), 3);
        assert!(node[0].is_line());
        assert!(node[1].is_line());
        assert!(node[2].is_multichoice());
    }

    #[test]
    fn multichoice_wrapper_adds_gather_line_if_present() {
        let choice1 = get_empty_choice(1);
        let gather1 = get_empty_gather(1);

        let lines_without_gather = vec![
            choice1.clone(),
            choice1.clone(),
            choice1.clone(),
        ];

        let mut index = 0;

        let (root, line) = parse_multichoice_with_gather(&mut index, 1, &lines_without_gather);

        assert!(root.is_multichoice());
        assert_eq!(root.len(), 3);
        assert!(line.is_none());

        let lines_with_gather = vec![
            choice1.clone(),
            choice1.clone(),
            gather1.clone(),
            choice1.clone(),
        ];

        index = 0;

        let (root, line) = parse_multichoice_with_gather(&mut index, 1, &lines_with_gather);

        assert!(root.is_multichoice());
        assert_eq!(root.len(), 2);
        assert!(line.unwrap().is_line());
    }

    #[test]
    fn multichoice_wrapper_increments_index_if_gather() {
        let choice1 = get_empty_choice(1);
        let gather1 = get_empty_gather(1);

        let lines_without_gather = vec![
            choice1.clone(),
        ];

        let mut index = 0;

        parse_multichoice_with_gather(&mut index, 1, &lines_without_gather);

        assert_eq!(index, 1);

        let lines_with_gather = vec![
            choice1.clone(),
            gather1.clone(),
        ];

        index = 0;

        parse_multichoice_with_gather(&mut index, 1, &lines_with_gather);

        assert_eq!(index, 2);
    }

    #[test]
    fn gathers_end_multichoices_at_same_level() {
        let choice1 = get_empty_choice(1);
        let gather1 = get_empty_gather(1);

        let lines = vec![
            choice1.clone(),
            choice1.clone(),
            gather1.clone(),
            choice1.clone(),
        ];

        let mut index = 0;

        let root = parse_multichoice(&mut index, 1, &lines);

        assert_eq!(index, 2);
        assert_eq!(root.len(), 2);
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

        let (root, _) = parse_multichoice_with_gather(&mut index, 1, &lines);

        assert_eq!(index, 9);
        assert_eq!(root.len(), 3);
    }

    #[test]
    fn parse_node_with_two_initial_items_then_nest_into_a_multichoice() {
        let line = get_line("");
        let choice1 = get_empty_choice(1);

        let lines = vec![line.clone(), line.clone(), choice1.clone(), choice1.clone()];

        let root = parse_full_node(&lines);

        assert_eq!(root.items.len(), 3);
        assert!(root.items[0].is_line());
        assert!(root.items[1].is_line());
        assert!(root.items[2].is_multichoice());
    }

    #[test]
    fn parse_node_with_a_gather_adds_line() {
        let line = get_line("");
        let choice1 = get_empty_choice(1);
        let choice2 = get_empty_choice(2);
        let gather1 = get_empty_gather(1);
        let gather2 = get_empty_gather(2);

        let lines = vec![
            line.clone(), 
            choice1.clone(), // Multichoice starts here as second level-1 element 
            choice2.clone(), // First element of level-2 multichoice
            choice2.clone(),
            gather2.clone(), // Breaks level-2 multichoice; becomes second element
            choice1.clone(),
            gather2.clone(),
            choice1.clone(),
            gather1.clone(), // Breaks multichoice; becomes third level-1 element
            choice1.clone(), // Fourth level-1 element due to gather
        ];

        let root = parse_full_node(&lines);

        assert_eq!(root.items.len(), 4);
        assert!(root.items[1].is_multichoice());
        assert!(root.items[2].is_line());
        assert!(root.items[3].is_multichoice());

        assert_eq!(root.items[1][0].len(), 2);
        assert!(root.items[1][0][1].is_line());
    }

    /***************************************************
     * Helper functions to do test assertion and debug *
     ***************************************************/

    impl DialogueNode {
        // Return a string representation of the entire graph of nodes.
        fn display(&self) -> String {
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
        fn display(&self) -> String {
            let mut buffer = String::new();

            self.display_indent(&mut buffer, 0);

            buffer
        }

        // Recursively descend into children, writing their structure into the buffer
        // with indents added for every level.
        fn display_indent(&self, buffer: &mut String, level: usize) {
            let indent = format!("{:width$}", ' ', width = 4 * level);

            match self {
                NodeItem::Line(line) => {
                    let s = format!("{indent}Line(\"{}\")\n", &line.text, indent = indent);
                    buffer.push_str(&s);
                }

                NodeItem::Node { kind, node } => {
                    let variant = match kind {
                        NodeType::MultiChoice => "MultiChoice [\n",
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
        fn len(&self) -> usize {
            match self {
                NodeItem::Node { node, .. } => node.items.len(),
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }

        // If `Self` is `NodeItem::Node`, return the boxed `Node`.
        // Panics if `Self` is not `Node`.
        fn node(&self) -> &DialogueNode {
            match self {
                NodeItem::Node { node, .. } => &node,
                _ => panic!("expected a `Node` but found {:?}", self),
            }
        }

        // Return `true` if `Self` is both `NodeItem::Node` and its kind is `NodeType::Choice`.
        fn is_choice(&self) -> bool {
            match self {
                NodeItem::Node {
                    kind: NodeType::Choice(..),
                    ..
                } => true,
                _ => false,
            }
        }

        // Return `true` if `Self` is `NodeItem::Line`.
        fn is_line(&self) -> bool {
            match self {
                NodeItem::Line(..) => true,
                _ => false,
            }
        }

        // Return `true` if `Self` is both `NodeItem::Node` and its kind is `NodeType::MultiChoice`.
        fn is_multichoice(&self) -> bool {
            match self {
                NodeItem::Node {
                    kind: NodeType::MultiChoice,
                    ..
                } => true,
                _ => false,
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
}
