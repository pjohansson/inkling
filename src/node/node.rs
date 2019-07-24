use crate::line::{ChoiceData, ParsedLine};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use super::parse::new_parse_full_node;

use crate::line::{Line, LineBuilder};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub struct RootNode {
    pub items: Vec<Container>,
    pub num_visited: u32,
}

impl RootNode {
    /// Parse a set of `ParsedLine` items and create a full graph representation of it.
    pub fn from_lines(lines: &[ParsedLine]) -> Self {
        new_parse_full_node(lines)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub struct Branch {
    pub choice: ChoiceData,
    pub items: Vec<Container>,
    pub num_visited: u32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub enum Container {
    Line(Line),
    BranchingChoice(Vec<Branch>),
}

pub struct RootNodeBuilder {
    items: Vec<Container>,
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
        self.add_item(Container::BranchingChoice(branching_set));
    }

    pub fn add_item(&mut self, item: Container) {
        self.items.push(item);
    }
    
    pub fn add_line(&mut self, line: Line) {
        self.add_item(Container::Line(line));
    }

    #[cfg(test)]
    pub fn with_branching_choice(mut self, branching_choice_set: Container) -> Self {
        self.items.push(branching_choice_set);
        self
    }

    #[cfg(test)]
    pub fn with_line_text(mut self, content: &str) -> Self {
        let line = Container::Line(LineBuilder::new().with_text(content).unwrap().build());
        self.items.push(line);
        self
    }
}

pub struct BranchBuilder {
    choice: ChoiceData,
    items: Vec<Container>,
    num_visited: u32,
}

impl BranchBuilder {
    pub fn from_choice(choice: ChoiceData) -> Self {
        BranchBuilder {
            choice,
            items: Vec::new(),
            num_visited: 0,
        }
    }

    pub fn add_branching_choice(&mut self, branching_set: Vec<Branch>) {
        self.add_item(Container::BranchingChoice(branching_set));
    }

    pub fn add_item(&mut self, item: Container) {
        self.items.push(item);
    }

    pub fn add_line(&mut self, line: Line) {
        self.add_item(Container::Line(line));
    }

    #[cfg(test)]
    pub fn with_branching_choice(mut self, branching_choice_set: Container) -> Self {
        self.items.push(branching_choice_set);
        self
    }

    #[cfg(test)]
    pub fn with_line_text(mut self, content: &str) -> Self {
        let line = Container::Line(LineBuilder::new().with_text(content).unwrap().build());
        self.items.push(line);
        self
    }

    pub fn build(mut self) -> Branch {
        let line = LineBuilder::new()
            .with_line(self.choice.line.clone())
            .build();

        let mut items = vec![Container::Line(line)];
        items.append(&mut self.items);

        Branch {
            choice: self.choice,
            items,
            num_visited: self.num_visited,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    impl Container {
        pub fn is_branching_choice(&self) -> bool {
            match self {
                Container::BranchingChoice(..) => true,
                _ => false,
            }
        }

        pub fn is_line(&self) -> bool {
            match self {
                Container::Line(..) => true,
                _ => false,
            }
        }
    }

    pub struct BranchingChoiceBuilder {
        items: Vec<Branch>,
    }

    impl BranchingChoiceBuilder {
        pub fn new() -> Self {
            BranchingChoiceBuilder { items: Vec::new() }
        }

        pub fn add_branch(mut self, choice: Branch) -> Self {
            self.items.push(choice);
            self
        }

        pub fn build(self) -> Container {
            Container::BranchingChoice(self.items)
        }
    }
}
