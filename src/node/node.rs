use crate::line::{Choice, Line, ParsedLine};

use super::parse::parse_full_node;

#[derive(Debug)]
/// Node in a graph representation of a dialogue tree.
pub struct DialogueNode {
    /// Children of current node.
    pub items: Vec<NodeItem>,
}

impl DialogueNode {
    /// Parse a set of `ParsedLine` items and create a full graph representation of it.
    pub fn from_lines(lines: &[ParsedLine]) -> Self {
        parse_full_node(lines)
    }
}

#[derive(Debug)]
pub enum NodeItem {
    /// Regular line of marked up text.
    Line(Line),
    /// Nested node, either a `ChoiceSet` which has `Choice`s as children, or a
    /// `Choice` which has more `Line`s and possibly further `ChoiceSet`s.
    Node {
        kind: NodeType,
        node: Box<DialogueNode>,
    },
}

#[derive(Debug)]
pub enum NodeType {
    /// Root of a set of choices. All node items will be of type `Choice`.
    ChoiceSet,
    /// Choice in a set of choices. All node items will be lines or further `ChoiceSet` nodes.
    Choice(Choice),
}
