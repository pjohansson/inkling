//! Errors from running `inkling`.

use std::{error::Error, fmt};

use crate::{follow::ChoiceInfo, knot::Address, node::Stack, story::Choice};

#[derive(Clone, Debug)]
/// Errors from running a story.
///
/// This struct mostly concerns errors which will be encountered due to some mistake
/// with the story or user input.
///
/// `OutOfChoices` and `OutOfContent` are runtime errors from the story running out
/// of content to display. This is likely due to the story returning to a single knot
/// or stitch multiple times, consuming all of its choices if no fallback choice has
/// been added. These issues should be taken into account when writing the story:
/// if content will be returned to it is important to keep track of how many times
/// this is allowed to happen, or have a fallback in place.
///
/// All internal errors are contained in the `Internal` variant. These concern everything
/// that went wrong due to some issue within `inkling` itself. If you encounter any,
/// please open an issue on Github.
pub enum InklingError {
    /// Internal errors caused by `inkling`.
    Internal(InternalError),
    /// An invalid choice index was given to resume the story with.
    InvalidChoice {
        /// Choice input by the user to resume the story with.
        selection: usize,
        /// List of choices that were available for the selection
        presented_choices: Vec<Choice>,
    },
    /// Tried to move to an invalid knot or stitch.
    InvalidMove {
        knot: String,
        stitch: Option<String>,
    },
    /// No choices or fallback choices were available in a story branch at the given address.
    OutOfChoices { address: Address },
    /// No content was available for the story to continue from.
    OutOfContent,
    /// Tried to resume a story that has not been started.
    ResumeBeforeStart,
    /// Tried to resume a story with a choice, but no choice was prompted.
    ResumeWithoutChoice,
    /// Tried to `start` a story that is already in progress.
    StartOnStoryInProgress,
}

#[derive(Clone, Debug)]
/// Internal errors from `inkling`.
///
/// These are errors which arise when the library produces objects, trees, text
/// or internal stacks that are inconsistent with each other or themselves.
///
/// If the library is well written these should not possibly occur; at least until
/// this point every part of the internals are fully deterministic. That obviously
/// goes for a lot of buggy code that has been written since forever, so nothing
/// unique there.
///
/// Either way, all those sorts of errors are encapsulated here. They should never
/// be caused by invalid user input or Ink files, those errors should be captured
/// by either the parent [`InklingError`][crate::error::InklingError]
/// or parsing [`ParseError`][crate::error::ParseError] error structures.
pub enum InternalError {
    /// The internal stack of knots is inconsistent or has not been set properly.
    BadKnotStack(StackError),
    /// Could not `Process` a line of text into its final form.
    CouldNotProcess(ProcessError),
    /// Selected branch index does not exist.
    IncorrectChoiceIndex {
        selection: usize,
        available_choices: Vec<ChoiceInfo>,
        stack_index: usize,
        stack: Stack,
    },
    /// Current stack is not properly representing the graph or has some indexing problems.
    IncorrectNodeStack(IncorrectNodeStackError),
    /// Tried to use an unvalidated address after the story was parsed.
    UseOfUnvalidatedAddress { address: Address },
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
    [Internal, InternalError]
];

impl_from_error![
    InternalError;
    [BadKnotStack, StackError],
    [CouldNotProcess, ProcessError],
    [IncorrectNodeStack, IncorrectNodeStackError]
];

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
            InvalidChoice {
                selection,
                presented_choices,
            } => write!(
                f,
                "Invalid selection of choice: selection was {} but number of choices was {} \
                 (maximum selection index is {})",
                selection,
                presented_choices.len(),
                presented_choices.len() - 1
            ),
            InvalidMove { knot, stitch } => match stitch {
                Some(stitch_name) => write!(
                    f,
                    "Invalid move: knot '{}' does not contain a stitch named '{}'",
                    knot, stitch_name
                ),
                None => write!(
                    f,
                    "Invalid move: story does not contain a knot name '{}'",
                    knot
                ),
            },
            OutOfChoices {
                address: Address::Validated { knot, stitch },
            } => write!(
                f,
                "Story reached a branching choice with no available choices to present \
                 or default choices to fall back on (knot: {}, stitch: {})",
                knot, stitch
            ),
            OutOfChoices { address } => write!(
                f,
                "Tried to use a non-validated `Address` ('{:?}') when following a story",
                address
            ),
            OutOfContent => write!(f, "Story ran out of content before an end was reached"),
            ResumeBeforeStart => write!(f, "Tried to resume a story that has not yet been started"),
            ResumeWithoutChoice => write!(
                f,
                "Tried to resume a story with a choice after moving to a new knot, where \
                 no choices have yet been encountered"
            ),
            StartOnStoryInProgress => {
                write!(f, "Called `start` on a story that is already in progress")
            }
        }
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use IncorrectNodeStackError::*;
        use InternalError::*;
        use ProcessErrorKind::*;
        use StackError::*;

        match self {
            BadKnotStack(err) => match err {
                BadAddress {
                    address: Address::Validated { knot, stitch },
                } => write!(
                    f,
                    "The currently set knot address (knot: {}, stitch: {}) does not \
                     actually represent a knot in the story",
                    knot, stitch
                ),
                BadAddress { address } => write!(
                    f,
                    "Tried to used a non-validated `Address` ('{:?}') in a function",
                    address
                ),
                NoLastChoices => write!(
                    f,
                    "Tried to follow with a choice but the last set of presented choices has \
                     not been saved"
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
            CouldNotProcess(ProcessError { kind }) => match kind {
                InvalidAlternativeIndex => write!(
                    f,
                    "When processing an alternative, an invalid index was used to pick an item"
                ),
                InklingError(err) => write!(f, "{}", err),
            },
            IncorrectChoiceIndex {
                selection,
                ref available_choices,
                stack_index,
                ref stack,
            } => write!(
                f,
                "Tried to resume after a choice was made but the chosen index does not exist \
                 in the set of choices. Somehow a faulty set of choices was created from this \
                 branch point and returned upwards, the stack is wrong, or the wrong set of \
                 choices was used elsewhere in the preparation of the choice list. \
                 Selection index: {}, number of branches: {} \
                 (node level: {}, stack: {:?})",
                selection,
                available_choices.len(),
                stack_index,
                stack
            ),
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
            UseOfUnvalidatedAddress { address } => {
                write!(f, "Tried to use unvalidated address '{:?}'", address)
            }
        }
    }
}

#[derive(Clone, Debug)]
/// Error from processing content into its final format.
pub struct ProcessError {
    /// Error variant.
    pub kind: ProcessErrorKind,
}

impl From<InklingError> for ProcessError {
    fn from(err: InklingError) -> Self {
        ProcessError {
            kind: ProcessErrorKind::InklingError(Box::new(err)),
        }
    }
}

#[derive(Clone, Debug)]
/// Variant of `ProcessError`.
pub enum ProcessErrorKind {
    /// An `Alternative` sequence tried to access an item with an out-of-bounds index.
    InvalidAlternativeIndex,
    /// An `InklingError` encountered during processing.
    InklingError(Box<InklingError>),
}

#[derive(Clone, Debug)]
/// Errors related to the stack of `Knots`, `Stitches` and choices set to
/// the [`Story`][crate::story::Story].
pub enum StackError {
    /// The current stack of `Address`es is empty and a follow was requested.
    NoStack,
    /// An invalid address was used inside the system.
    ///
    /// This means that some bad assumptions have been made somewhere. Addresses are
    /// always supposed to be verified as valid before use.
    BadAddress { address: Address },
    /// No set of presented choices have been added to the system.
    NoLastChoices,
    /// No root knot was added to the stack when the `Story` was constructed.
    NoRootKnot { knot_name: String },
}

#[derive(Clone, Debug)]
/// Current node tree [`Stack`][crate::node::Stack] is incorrect.
pub enum IncorrectNodeStackError {
    /// Tried to follow through nodes with an empty stack.
    EmptyStack,
    /// Found a `Line` object where a set of branching choices should be.
    ExpectedBranchingPoint { stack_index: usize, stack: Stack },
    /// Tried to follow a branch but stack does not have an index for the follow,
    /// it is too short.
    MissingBranchIndex { stack_index: usize, stack: Stack },
    /// Stack contains an invalid index for the current node level.
    OutOfBounds {
        stack_index: usize,
        stack: Stack,
        num_items: usize,
    },
}
