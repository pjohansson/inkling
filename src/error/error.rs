//! Internal errors from `inkling` itself.

use crate::{
    line::ChoiceData,
    node::{NodeItem, NodeType, Stack},
    story::Choice,
};

use std::{error::Error, fmt};

#[derive(Clone, Debug)]
/// Internal error from walking through a story.
///
/// Most likely due to the `DialogueNode` tree of a story being constructed incorrectly,
/// which will be due to a logical error in the set-up code since the user has no
/// control over it.
pub enum InklingError {
    /// The graph of `DialogueNode`s has an incorrect structure. This can be that `Choice`s
    /// are not properly nested under `ChoiceSet` nodes.
    BadGraph(BadGraphKind),
    /// The current stack is not properly representing the graph or has some indexing problems.
    IncorrectStack {
        kind: IncorrectStackKind,
        stack: Stack,
    },
    /// A choice was made with an internal index that does not match one existing in the set.
    /// Means that the choice set presented to the user was not created to represent the set
    /// of encountered choices, or that somehow a faulty choice was returned to continue
    /// the story with.
    InvalidChoice {
        /// Index of choice that was used internally when the choice was not found.
        index: usize,
        /// Choice input by the user to resume the story with.
        choice: Option<Choice>,
        /// List of choices that were available for the selection and if they were given
        /// to the user in the `Prompt::Choice` set.
        presented_choices: Vec<(bool, Choice)>,
        /// List of all choices that were available in their internal representation.
        internal_choices: Vec<ChoiceData>,
    },
    /// No root knot has been set to begin following the story from.
    NoKnotStack,
    /// Tried to resume a story that has not been started.
    ResumeBeforeStart,
    /// Tried to `start` a story that is already in progress.
    StartOnStoryInProgress,
    /// The story tried to move to a knot that doesn't exist.
    UnknownKnot { knot_name: String },
}

impl fmt::Display for InklingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use IncorrectStackKind::*;
        use InklingError::*;

        match self {
            BadGraph(BadGraphKind::ExpectedNode {
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
            InvalidChoice {
                index,
                choice,
                presented_choices,
                ..
            } => {
                let presented_choices_string = presented_choices
                    .iter()
                    .map(|(shown, choice)| {
                        if *shown {
                            format!("{:?} (shown as available)", choice)
                        } else {
                            format!("{:?} (not shown)", choice)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                match choice {
                    Some(choice) => {
                        write!(f,
                        "Tried to resume the story with an invalid choice: input choice was {:?}, \
                        while available choices were: \n
                        {}",
                        choice, presented_choices_string
                        )
                    }
                    None => write!(
                        f,
                        "Tried to resume the story with an invalid choice: \
                         input choice cannot be found but its internal index was {}, \
                         available choices were: [{}]",
                        index, presented_choices_string
                    ),
                }
            }
            NoKnotStack => write!(
                f,
                "`Story` object was created but no root `Knot` was set to start \
                 following the story from"
            ),
            ResumeBeforeStart => write!(f, "Tried to resume a story that has not yet been started"),
            StartOnStoryInProgress => {
                write!(f, "Called `start` on a story that is already in progress")
            }
            UnknownKnot { knot_name } => write!(
                f,
                "Tried to follow a knot with name {} but no such knot exists",
                knot_name
            ),
            IncorrectStack { kind, stack } => match kind {
                BadIndices {
                    node_level,
                    index,
                    num_items,
                } => write!(
                    f,
                    "Current stack has invalid index {} at node level {}: size of set is {} \
                     (stack: {:?})",
                    index, node_level, num_items, stack
                ),
                EmptyStack => write!(f, "Tried to advance through a knot with an empty stack"),
                Gap { node_level } => write!(
                    f,
                    "Current stack is too short for the current node level {}: \
                     cannot get or add a stack index because stack indices for one or more \
                     prior nodes are missing, which means the stack is incorrect (stack: {:?})",
                    node_level, stack
                ),
                MissingIndices { node_level, kind } => {
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
                NotTruncated { node_level } => write!(
                    f,
                    "Current stack is not truncated to the current node level {} (stack: {:?})",
                    node_level, stack
                ),
            },
        }
    }
}

impl Error for InklingError {}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
/// Whether the parent or child index caused an error when walking through a `DialogueNode` tree.
pub enum WhichIndex {
    Parent,
    Child,
}
