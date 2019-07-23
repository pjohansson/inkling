mod follow;
#[allow(dead_code)]
mod node;
#[allow(dead_code)]
mod parse;

pub use follow::Stack;
pub use node::*;
pub use node::{DialogueNode, NodeItem, NodeType};
