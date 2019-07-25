use crate::{
    line::{InternalChoice, InternalLine, ParsedLineKind},
    node::parse_root_node,
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Root of a single `Stitch`, containing all text and branching content belonging to it.
pub struct RootNode {
    pub items: Vec<NodeItem>,
    pub num_visited: u32,
}

impl RootNode {
    /// Parse a set of `ParsedLine` items and create a full graph representation of it.
    pub fn from_lines(lines: &[ParsedLineKind]) -> Self {
        parse_root_node(lines)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Branch from a set of choices in a `Stitch`. Largely identical to `RootNode`
/// but also contains the data associated with the choice leading to it.
pub struct Branch {
    pub choice: InternalChoice,
    pub items: Vec<NodeItem>,
    pub num_visited: u32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Every item that a `Stitch` contains can be either some text producing asset
/// or a branching point which the user must select an option from to continue.
pub enum NodeItem {
    Line(InternalLine),
    BranchingPoint(Vec<Branch>),
}

#[cfg(test)]
/// Simplified checking of which match a `NodeItem` is during testing.
impl NodeItem {
    pub fn is_branching_choice(&self) -> bool {
        match self {
            NodeItem::BranchingPoint(..) => true,
            _ => false,
        }
    }

    pub fn is_line(&self) -> bool {
        match self {
            NodeItem::Line(..) => true,
            _ => false,
        }
    }
}

pub mod builders {
    use super::{Branch, NodeItem, RootNode};

    use crate::line::{InternalChoice, InternalLine};

    #[cfg(test)]
    use crate::line::LineChunk;

    /// Builder for a `RootNote`.
    ///
    /// # Notes
    ///  *  By default sets `num_visited` to 0.
    pub struct RootNodeBuilder {
        items: Vec<NodeItem>,
        num_visited: u32,
    }

    impl RootNodeBuilder {
        pub fn new() -> Self {
            RootNodeBuilder {
                items: Vec::new(),
                num_visited: 0,
            }
        }

        pub fn build(self) -> RootNode {
            RootNode {
                items: self.items,
                num_visited: self.num_visited,
            }
        }

        pub fn add_branching_choice(&mut self, branching_set: Vec<Branch>) {
            self.add_item(NodeItem::BranchingPoint(branching_set));
        }

        pub fn add_item(&mut self, item: NodeItem) {
            self.items.push(item);
        }

        pub fn add_line(&mut self, line: InternalLine) {
            self.add_item(NodeItem::Line(line));
        }

        #[cfg(test)]
        pub fn with_item(mut self, item: NodeItem) -> Self {
            self.items.push(item);
            self
        }

        #[cfg(test)]
        pub fn with_branching_choice(self, branching_choice_set: NodeItem) -> Self {
            self.with_item(branching_choice_set)
        }

        #[cfg(test)]
        pub fn with_line_chunk(self, chunk: LineChunk) -> Self {
            self.with_item(NodeItem::Line(InternalLine::from_chunk(chunk)))
        }

        #[cfg(test)]
        pub fn with_text_line_chunk(self, content: &str) -> Self {
            self.with_item(NodeItem::Line(InternalLine::from_string(content)))
        }
    }

    /// Builder for a `Branch`, created from a `ChoiceData` that spawns the branch in
    /// the parsed lines of text content.
    ///
    /// # Notes
    ///  *  Adds the line from its choice as the first in its item list.
    pub struct BranchBuilder {
        choice: InternalChoice,
        items: Vec<NodeItem>,
        num_visited: u32,
    }

    impl BranchBuilder {
        pub fn from_choice(choice: InternalChoice) -> Self {
            let line = choice.display_text.clone();

            BranchBuilder {
                choice,
                items: vec![NodeItem::Line(line)],
                num_visited: 0,
            }
        }

        pub fn build(self) -> Branch {
            Branch {
                choice: self.choice,
                items: self.items,
                num_visited: self.num_visited,
            }
        }

        pub fn add_branching_choice(&mut self, branching_set: Vec<Branch>) {
            self.add_item(NodeItem::BranchingPoint(branching_set));
        }

        pub fn add_item(&mut self, item: NodeItem) {
            self.items.push(item);
        }

        pub fn add_line(&mut self, line: InternalLine) {
            self.add_item(NodeItem::Line(line));
        }

        #[cfg(test)]
        pub fn with_item(mut self, item: NodeItem) -> Self {
            self.items.push(item);
            self
        }

        #[cfg(test)]
        pub fn with_branching_choice(self, branching_choice_set: NodeItem) -> Self {
            self.with_item(branching_choice_set)
        }

        #[cfg(test)]
        pub fn with_text_line_chunk(self, content: &str) -> Self {
            self.with_item(NodeItem::Line(InternalLine::from_string(content)))
        }
    }

    #[cfg(test)]
    pub struct BranchingPointBuilder {
        items: Vec<Branch>,
    }

    #[cfg(test)]
    impl BranchingPointBuilder {
        pub fn new() -> Self {
            BranchingPointBuilder { items: Vec::new() }
        }

        pub fn with_branch(mut self, choice: Branch) -> Self {
            self.items.push(choice);
            self
        }

        pub fn build(self) -> NodeItem {
            NodeItem::BranchingPoint(self.items)
        }
    }
}
