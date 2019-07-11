use std::{error::Error, fmt};

pub use internal::InternalError;
pub(crate) use internal::*;

#[derive(Debug)]
/// Error from walking through a story.
pub enum FollowError {
    /// Issue with walking through the story due to some internal inconsistency.
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

mod internal {
    //! Internal errors from `inkling` itself.

    use crate::node::{NodeItem, NodeType, Stack};
    use std::{error::Error, fmt};

    #[derive(Debug)]
    /// Internal error from walking through a story.
    ///
    /// Most likely due to the `DialogueNode` tree of a story being constructed incorrectly,
    /// which will be due to a logical error in the set-up code since the user has no
    /// control over it.
    pub enum InternalError {
        /// The graph of `DialogueNode`s has an incorrect structure. This can be that `Choice`s
        /// are not properly nested under `ChoiceSet` nodes.
        BadGraph(BadGraphKind),
        /// The current stack is not properly representing the graph or has some indexing problems.
        IncorrectStack {
            kind: IncorrectStackKind,
            stack: Stack,
        },
        /// No root knot has been set to begin following the story from.
        NoKnotStack,
        /// The story tried to move to a knot that doesn't exist.
        UnknownKnot { name: String },
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
                InternalError::NoKnotStack => write!(
                    f,
                    "`Story` object was created but no root `Knot` was set to start \
                     following the story from"
                ),
                InternalError::UnknownKnot { name } => write!(
                    f,
                    "Tried to follow a knot with name {} but no such knot exists",
                    name
                ),
                InternalError::IncorrectStack { kind, stack } => match kind {
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
                    IncorrectStackKind::EmptyStack => {
                        write!(f, "Tried to advance through a knot with an empty stack")
                    }
                    IncorrectStackKind::Gap { node_level } => write!(
                        f,
                        "Current stack is too short for the current node level {}: \
                         cannot get or add a stack index because stack indices for one or more \
                         prior nodes are missing, which means the stack is incorrect (stack: {:?})",
                        node_level, stack
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
                    IncorrectStackKind::NotTruncated { node_level } => write!(
                        f,
                        "Current stack is not truncated to the current node level {} (stack: {:?})",
                        node_level, stack
                    ),
                },
            }
        }
    }

    impl Error for InternalError {}

    #[derive(Debug)]
    /// Error variant associated with the `DialogueNode` graph being poorly constructed.
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
    /// Error variant associated with the stack created when walking through a `DialogueNode`
    /// tree being poorly constructed.
    pub enum IncorrectStackKind {
        /// Stack contains an invalid index for the current node level.
        BadIndices {
            node_level: usize,
            index: usize,
            num_items: usize,
        },
        /// Tried to follow through nodes with an empty stack.
        EmptyStack,
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
    /// Whether the parent or child index caused an error when walking through a `DialogueNode` tree.
    pub enum WhichIndex {
        Parent,
        Child,
    }
}
