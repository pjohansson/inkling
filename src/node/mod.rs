//! Nested node structure for branching story content.
//!
//! Of main interest in this module is the [`RootNode`][crate::node::RootNode]
//! and [`Branch`][crate::node::Branch] items which contain the nested story structure,
//! and the [`Follow`][crate::node::Follow] trait which allows us to walk through
//! their content.

mod follow;
mod node;
mod parse;

pub use follow::{Follow, Stack};
pub(self) use node::builders;
pub use node::{builders::RootNodeBuilder, Branch, NodeItem, RootNode};
pub(self) use parse::parse_root_node;
