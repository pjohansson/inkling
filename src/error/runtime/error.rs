//! Errors from running `inkling`.

use std::{error::Error, fmt};

use crate::{
    error::{runtime::internal::StackError, variable::VariableError, InternalError},
    line::Variable,
    story::{Choice, Location},
};

impl Error for InklingError {}

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
    /// Tried to assign a new value to a CONST variable.
    AssignedToConst {
        name: String,
    },
    /// Internal errors caused by `inkling`.
    Internal(InternalError),
    /// Use of a `Location` which does not exist in the story.
    InvalidAddress {
        location: Location,
    },
    /// An invalid choice index was given to resume the story with.
    InvalidChoice {
        /// Choice input by the user to resume the story with.
        selection: usize,
        /// List of choices that were available for the selection
        presented_choices: Vec<Choice>,
    },
    /// Used a variable name that is not present in the story as an input variable.
    InvalidVariable {
        name: String,
    },
    /// Called `make_choice` when no choice had been requested.
    ///
    /// Likely directly at the start of a story or after a `move_to` call was made.
    MadeChoiceWithoutChoice,
    /// No choices or fallback choices were available in a story branch at the given location.
    OutOfChoices {
        location: Location,
    },
    /// No content was available for the story to continue from.
    OutOfContent,
    /// Tried to print a variable that cannot be printed.
    PrintInvalidVariable {
        name: String,
        value: Variable,
    },
    VariableError(VariableError),
}

impl From<StackError> for InklingError {
    fn from(err: StackError) -> Self {
        InklingError::Internal(InternalError::BadKnotStack(err))
    }
}

impl_from_error![
    InklingError;
    [Internal, InternalError],
    [VariableError, VariableError]
];

impl fmt::Display for InklingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use InklingError::*;

        match self {
            AssignedToConst { name } => {
                write!(f, "Tried to assign a value to CONST variable '{}'", name)
            }
            Internal(err) => write!(f, "INTERNAL ERROR: {}", err),
            InvalidAddress {
                location: Location { knot, stitch },
            } => match stitch {
                Some(stitch_name) => write!(
                    f,
                    "Invalid address: knot '{}' does not contain a stitch named '{}'",
                    knot, stitch_name
                ),
                None => write!(
                    f,
                    "Invalid address: story does not contain a knot name '{}'",
                    knot
                ),
            },
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
            InvalidVariable { name } => write!(
                f,
                "Invalid variable: no variable with  name '{}' exists in the story",
                name
            ),
            MadeChoiceWithoutChoice => write!(
                f,
                "Tried to make a choice, but no choice is currently active. Call `resume` \
                 and assert that a branching choice is returned before calling this again."
            ),
            OutOfChoices {
                location: Location { knot, stitch },
            } => {
                write!(
                    f,
                    "Story reached a branching choice with no available choices to present \
                    or default choices to fall back on (knot: {}",
                    knot
                )?;

                if let Some(name) = stitch {
                    write!(f, ", stitch: {}", name)?;
                }

                write!(f, ")")
            }
            OutOfContent => write!(f, "Story ran out of content before an end was reached"),
            PrintInvalidVariable { name, value } => write!(
                f,
                "Cannot print variable '{}' which has value '{:?}': invalid type",
                name, value
            ),
            VariableError(err) => write!(f, "{}", err),
        }
    }
}
