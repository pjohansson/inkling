//! Internal errors from `inkling` itself.

use crate::{
    follow::*,
    line::ProcessError,
    line::*,
    node::Stack,
    story::{Address, Choice},
};

use std::{error::Error, fmt};

#[derive(Clone, Debug)]
/// Internal error from walking through a story.
///
/// Most likely due to the `DialogueNode` tree of a story being constructed incorrectly,
/// which will be due to a logical error in the set-up code since the user has no
/// control over it.
pub enum InklingError {
    /// Internal errors caused by `inkling`.
    Internal(InternalError),
    /// An invalid address was encountered when following the story.
    InvalidAddress(InvalidAddressError),
    /// A choice was made with an internal index that does not match one existing in the set.
    /// This means that the choice set presented to the user was not created to represent the set
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
        internal_choices: Vec<ChoiceExtra>,
    },
    /// No choices or fallback choices were available in a story branch at the given address.
    OutOfChoices {
        address: Address,
    },
    /// No content was available for the story to continue from.
    OutOfContent,
    /// Tried to resume a story that has not been started.
    ResumeBeforeStart,
    /// Tried to `start` a story that is already in progress.
    StartOnStoryInProgress,
    ProcessError,
}

#[derive(Clone, Debug)]
/// A divert (or other address) in the story is invalid.
pub enum InvalidAddressError {
    /// The address is not formatted correctly.
    BadFormat { line: String },
    /// The address references a `Knot` that is not in the story.
    UnknownKnot { knot_name: String },
    /// The address references a `Stitch` that is not present in the current `Knot`.
    UnknownStitch {
        knot_name: String,
        stitch_name: String,
    },
}

#[derive(Clone, Debug)]
pub enum InternalError {
    /// The internal stack of knots is inconsistent or has not been set properly.
    BadKnotStack(StackError),
    /// The current stack is not properly representing the graph or has some indexing problems.
    IncorrectNodeStack(IncorrectNodeStackError),
}

impl Error for InklingError {}
impl Error for InternalError {}

/// Wrapper to implement From for variants when the variant is simply encapsulated
/// in the enum.
///
/// # Example
/// Running
/// ```
/// impl_from_error[
///     MyError,
///     [Variant, ErrorData]
/// ];
/// ```
/// is identical to running
/// ```
/// impl From<ErrorData> for MyError {
///     from(err: ErrorData) -> Self {
///         Self::Variant(err)
///     }
/// }
/// ```
/// The macro can also implement several variants at once:
/// ```
/// impl_from_error[
///     MyError,
///     [Variant1, ErrorData1],
///     [Variant2, ErrorData2]
/// ];
/// ```
macro_rules! impl_from_error {
    ($for_type:ident; $([$variant:ident, $from_type:ident]),+) => {
        $(
            impl From<$from_type> for $for_type {
                fn from(err: $from_type) -> Self {
                    $for_type::$variant(err)
                }
            }
        )*
    }
}

impl_from_error![
    InklingError;
    [Internal, InternalError],
    [InvalidAddress, InvalidAddressError]
];

impl_from_error![
    InternalError;
    [IncorrectNodeStack, IncorrectNodeStackError]
];

impl From<ProcessError> for InklingError {
    fn from(_: ProcessError) -> Self {
        InklingError::ProcessError
    }
}

impl From<StackError> for InklingError {
    fn from(err: StackError) -> Self {
        InklingError::Internal(InternalError::BadKnotStack(err))
    }
}

impl fmt::Display for InklingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use InklingError::*;

        match self {
            Internal(err) => write!(f, "INTERNAL ERROR: {}", err),
            InvalidAddress(err) => match err {
                InvalidAddressError::BadFormat { line } => write!(
                    f,
                    "Encountered an address '{}' that could not be parsed",
                    line
                ),
                InvalidAddressError::UnknownKnot { knot_name } => write!(
                    f,
                    "Tried to divert to a knot with name '{}', \
                     but no such knot exists in the story",
                    knot_name
                ),
                InvalidAddressError::UnknownStitch {
                    knot_name,
                    stitch_name,
                } => write!(
                    f,
                    "Tried to divert to stitch '{}' belonging to knot '{}', \
                     but no such stitch exists in the knot",
                    stitch_name, knot_name
                ),
            },
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
            OutOfChoices {
                address: Address { knot, stitch },
            } => write!(
                f,
                "Story reached a branching choice with no available choices to present \
                 or default choices to fall back on (knot: {}, stitch: {})",
                knot, stitch
            ),
            OutOfContent => write!(f, "Story ran out of content before an end was reached"),
            ResumeBeforeStart => write!(f, "Tried to resume a story that has not yet been started"),
            StartOnStoryInProgress => {
                write!(f, "Called `start` on a story that is already in progress")
            }
            ProcessError => unimplemented!(),
        }
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use IncorrectNodeStackError::*;
        use InternalError::*;
        use StackError::*;

        match self {
            BadKnotStack(err) => match err {
                BadAddress {
                    address: Address { knot, stitch },
                } => write!(
                    f,
                    "The currently set knot address (knot: {}, stitch: {}) does not \
                     actually represent a knot in the story",
                    knot, stitch
                ),
                NoRootKnot { knot_name } => write!(
                    f,
                    "After reading a set of knots, the root knot with name {} \
                     does not exist in the set",
                    knot_name
                ),
                NoStack => write!(
                    f,
                    "There is no currently set knot or address to follow the story from"
                ),
            },
            IncorrectNodeStack(err) => match err {
                EmptyStack => write!(f, "Tried to advance through a knot with an empty stack"),
                ExpectedBranchingPoint { stack_index, stack } => {
                    let item_number = stack[*stack_index];

                    write!(
                        f,
                        "While resuming a follow the stack found a regular line where \
                         it expected a branch point to nest deeper into. \
                         The stack has been corrupted. \
                         (stack level: {}, item number: {}, stack: {:?}",
                        stack_index, item_number, stack
                    )
                }
                MissingBranchIndex { stack_index, stack } => write!(
                    f,
                    "While resuming a follow the stack did not contain an index to \
                         select a branch with from a set of choices. The stack has been \
                         corrupted.
                         (stack level: {}, attempted index: {}, stack: {:?}",
                    stack_index,
                    stack_index + 1,
                    stack
                ),
                OutOfBounds {
                    stack_index,
                    stack,
                    num_items,
                } => write!(
                    f,
                    "Current stack has invalid index {} at node level {}: size of set is {} \
                     (stack: {:?})",
                    stack[*stack_index], stack_index, num_items, stack
                ),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub enum StackError {
    /// No stack has been set in the system, but a follow was requested. This should not happen.
    NoStack,
    /// An invalid address was used inside the system, which means that some bad assumptions
    /// have been made somewhere. Addresses are always supposed to be verified correct before
    /// use.
    BadAddress { address: Address },
    /// When creating the initial stack after constructing the knots, the root knot was not
    /// present in the set.
    NoRootKnot { knot_name: String },
}

#[derive(Clone, Debug)]
/// Error variant associated with the stack created when walking through a `DialogueNode`
/// tree being poorly constructed.
pub enum IncorrectNodeStackError {
    /// Tried to follow through nodes with an empty stack.
    EmptyStack,
    /// Found a `Line` object where a set of branching choices should be.
    ExpectedBranchingPoint { stack_index: usize, stack: Stack },
    /// Stack contains an invalid index for the current node level.
    OutOfBounds {
        stack_index: usize,
        stack: Stack,
        num_items: usize,
    },
    /// Tried to follow a branch but stack does not have an index for the follow,
    /// it is too short.
    MissingBranchIndex { stack_index: usize, stack: Stack },
}
