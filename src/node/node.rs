use crate::{
    line::{FullLine, LineBuilder, LineChunk, *},
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
    pub choice: FullChoice,
    pub items: Vec<NodeItem>,
    pub num_visited: u32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Every item that a `Stitch` contains can be either some text producing asset
/// or a branching point which the user must select an option from to continue.
pub enum NodeItem {
    Line(FullLine),
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
    use super::{Branch, FullChoice, FullLine, LineBuilder, LineChunk, NodeItem, RootNode};

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

        pub fn add_full_line(&mut self, line: FullLine) {
            self.add_item(NodeItem::Line(line));
        }

        pub fn add_line(&mut self, line: LineChunk) {
            let full_line = FullLine::from_chunk(line);
            self.add_item(NodeItem::Line(full_line));
        }

        #[cfg(test)]
        pub fn with_branching_choice(mut self, branching_choice_set: NodeItem) -> Self {
            self.items.push(branching_choice_set);
            self
        }

        #[cfg(test)]
        pub fn with_line_text(mut self, content: &str) -> Self {
            let chunk = LineBuilder::new().with_text(content).unwrap().build();
            let full_line = FullLine::from_chunk(chunk);
            self.items.push(NodeItem::Line(full_line));
            self
        }
    }

    /// Builder for a `Branch`, created from a `ChoiceData` that spawns the branch in
    /// the parsed lines of text content.
    ///
    /// # Notes
    ///  *  Adds the line from its choice as the first in its item list.
    pub struct BranchBuilder {
        choice: FullChoice,
        items: Vec<NodeItem>,
        num_visited: u32,
    }

    impl BranchBuilder {
        pub fn from_choice(choice: FullChoice) -> Self {
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

        pub fn add_full_line(&mut self, line: FullLine) {
            self.add_item(NodeItem::Line(line));
        }

        pub fn add_line(&mut self, chunk: LineChunk) {
            let full_line = FullLine::from_chunk(chunk);
            self.add_item(NodeItem::Line(full_line));
        }

        #[cfg(test)]
        pub fn with_branching_choice(mut self, branching_choice_set: NodeItem) -> Self {
            self.items.push(branching_choice_set);
            self
        }

        #[cfg(test)]
        pub fn with_line_text(mut self, content: &str) -> Self {
            let chunk = LineBuilder::new().with_text(content).unwrap().build();
            let full_line = FullLine::from_chunk(chunk);
            self.items.push(NodeItem::Line(full_line));
            self
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
