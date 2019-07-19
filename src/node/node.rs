use crate::line::{ChoiceData, LineData, ParsedLine};

use std::cell::Cell;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use super::parse::parse_full_node;

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Node in a graph representation of a dialogue tree.
pub struct DialogueNode {
    /// Children of current node.
    pub items: Vec<NodeItem>,
    pub num_visited: Cell<u32>,
}

impl DialogueNode {
    /// Parse a set of `ParsedLine` items and create a full graph representation of it.
    pub fn from_lines(lines: &[ParsedLine]) -> Self {
        parse_full_node(lines)
    }

    pub fn with_items(items: Vec<NodeItem>) -> Self {
        DialogueNode {
            items,
            num_visited: Cell::new(0),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub enum NodeItem {
    /// Regular line of marked up text.
    Line(LineData),
    /// Nested node, either a `ChoiceSet` which has `Choice`s as children, or a
    /// `Choice` which has more `Line`s and possibly further `ChoiceSet`s.
    Node {
        kind: NodeType,
        node: Box<DialogueNode>,
    },
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub enum NodeType {
    /// Root of a set of choices. All node items will be of type `Choice`.
    ChoiceSet,
    /// Choice in a set of choices. All node items will be lines or further `ChoiceSet` nodes.
    Choice(ChoiceData),
}
