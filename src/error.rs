use std::{error::Error, fmt};

use crate::node::{NodeItem, NodeType, Stack};

#[derive(Debug)]
pub enum FollowError {
    /// Issue with walking through the stack, either due to the graph of `DialogueNode`s
    /// being incorrect or the stack not being updated or constructed correctly.
    InternalError(InternalError),
    /// When prompted with a set of choices the user supplied a choice with an incorrect index.
    InvalidChoice {
        selection: usize,
        num_choices: usize,
    },
}

impl fmt::Display for FollowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FollowError::InvalidChoice {
                selection,
                num_choices,
            } => write!(
                f,
                "Invalid choice: selected choice with index {} but maximum is {}",
                selection,
                num_choices - 1
            ),
            FollowError::InternalError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for FollowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FollowError::InternalError(error) => Some(error),
            _ => None,
        }
    }
}

impl From<InternalError> for FollowError {
    fn from(err: InternalError) -> Self {
        FollowError::InternalError(err)
    }
}

#[derive(Debug)]
pub enum InternalError {
    /// The graph of `DialogueNode`s has an incorrect structure. This can be that `Choice`s
    /// are not properly nested under `ChoiceSet` nodes.
    BadGraph(BadGraphKind),
    /// The current stack is not properly representing the graph or has some indexing problems.
    IncorrectStack {
        kind: IncorrectStackKind,
        stack: Stack,
    },
    UnknownKnot {
        name: String,
    },
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InternalError::BadGraph(BadGraphKind::ExpectedNode {
                index,
                node_level,
                expected,
                found,
            }) => write!(
                f,
                "Expected a `DialogueNode` that is a {:?} but got a {:?} \
                 (node level: {}, index: {})",
                expected, found, node_level, index
            ),
            InternalError::UnknownKnot { name } => write!(
                f,
                "Tried to follow a knot with name {} but no such knot exists",
                name
            ),
            InternalError::IncorrectStack { kind, stack } => match kind {
                IncorrectStackKind::NotTruncated { node_level } => write!(
                    f,
                    "Current stack is not truncated to the current node level {} (stack: {:?})",
                    node_level, stack
                ),
                IncorrectStackKind::Gap { node_level } => write!(
                    f,
                    "Current stack is too short for the current node level {}: \
                     cannot get or add a stack index because stack indices for one or more \
                     prior nodes are missing, which means the stack is incorrect (stack: {:?})",
                    node_level, stack
                ),
                IncorrectStackKind::BadIndices {
                    node_level,
                    index,
                    num_items,
                } => write!(
                    f,
                    "Current stack has invalid index {} at node level {}: size of set is {} \
                     (stack: {:?})",
                    index, node_level, num_items, stack
                ),
                IncorrectStackKind::MissingIndices { node_level, kind } => {
                    let level = match kind {
                        WhichIndex::Parent => *node_level,
                        WhichIndex::Child => *node_level + 1,
                    };

                    write!(f, "Current stack has no index for node level {}", level)?;

                    if let WhichIndex::Child = kind {
                        write!(f, ", which was accessed as a child node during a follow")?;
                    }

                    write!(f, " (stack: {:?}", stack)
                }
            },
        }
    }
}

impl Error for InternalError {}

#[derive(Debug)]
pub enum BadGraphKind {
    /// Tried to access a `NodeItem` assuming that it was of a particular kind,
    /// but it was not.
    ExpectedNode {
        /// Index of item in parent list.
        index: usize,
        /// Level of parent `DialogueNode`.
        node_level: usize,
        /// Expected kind.
        expected: NodeItemKind,
        /// Encountered kind.
        found: NodeItemKind,
    },
}

#[derive(Debug)]
/// Simple representation of what a `NodeItem` is.
pub enum NodeItemKind {
    Line,
    Choice,
    ChoiceSet,
}

impl From<&NodeItem> for NodeItemKind {
    fn from(item: &NodeItem) -> Self {
        match item {
            NodeItem::Line(..) => NodeItemKind::Line,
            NodeItem::Node {
                kind: NodeType::Choice(..),
                ..
            } => NodeItemKind::Choice,
            NodeItem::Node {
                kind: NodeType::ChoiceSet,
                ..
            } => NodeItemKind::ChoiceSet,
        }
    }
}

#[derive(Debug)]
pub enum IncorrectStackKind {
    /// Stack contains an invalid index for the current node level.
    BadIndices {
        node_level: usize,
        index: usize,
        num_items: usize,
    },
    /// Stack has a gap from the last added node level and the current.
    Gap { node_level: usize },
    /// Stack is missing an index when walking through it, either for the current (parent)
    /// node or for a child node. The parent here *should* be a node which contains lines
    /// and possible choice sets, while the child will be a node in a choice set .
    MissingIndices { node_level: usize, kind: WhichIndex },
    /// Stack was not truncated before following into a new node.
    NotTruncated { node_level: usize },
}

#[derive(Debug)]
pub enum WhichIndex {
    Parent,
    Child,
}
