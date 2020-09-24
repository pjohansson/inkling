//! Data types of a story.

use crate::{
    error::{utils::MetaData, InklingError},
    line::Variable,
};

use std::collections::HashMap;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Single line of text in a story, ready to display.
pub struct Line {
    /// Text to display.
    ///
    /// The text is ready to be printed as-is, without the addition of more characters.
    /// It been processed to remove extraneous whitespaces and contains a newline character
    /// at the end of the line unless the line was glued to the next.
    pub text: String,
    /// Tags set to the line.
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Choice presented to the user.
pub struct Choice {
    /// Line of text to represent the choice with.
    ///
    /// The text is ready to be printed as-is. It is trimmed of whitespace from both ends
    /// and contains no newline character at the end.
    pub text: String,
    /// Tags associated with the choice.
    pub tags: Vec<String>,
    /// Internal index of choice in set.
    pub(crate) index: usize,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Result from following a `Story`.
///
/// # Examples
/// ```
/// # use inkling::{read_story_from_string, Prompt};
/// let content = "\
/// Professor Lidenbrock had barely a spattering of water left in his flask.
/// *   Axel got the last of it.
/// *   He pressed on, desperately hoping to find water soon.
/// ";
///
/// let mut story = read_story_from_string(content).unwrap();
/// let mut line_buffer = Vec::new();
///
/// match story.resume(&mut line_buffer).unwrap() {
///     Prompt::Choice(choice_set) => {
///         println!("Choose:");
///         for (i, choice) in choice_set.iter().enumerate() {
///             println!("{}. {}", i + 1, choice.text);
///         }
///     },
///     Prompt::Done => { /* the story reached its end */ },
/// }
/// ```
pub enum Prompt {
    /// The story reached an end.
    Done,
    /// A choice was encountered.
    Choice(Vec<Choice>),
}

impl Prompt {
    /// If a set of choices was returned, retrieve them without having to match.
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Prompt};
    /// let content = "\
    /// Professor Lidenbrock had barely a spattering of water left in his flask.
    /// *   Axel got the last of it.
    /// *   He pressed on, desperately hoping to find water soon.
    /// ";
    ///
    /// let mut story = read_story_from_string(content).unwrap();
    /// let mut line_buffer = Vec::new();
    ///
    /// if let Some(choices) = story.resume(&mut line_buffer).unwrap().get_choices() {
    ///     /* do what you want */
    /// }
    /// ```
    pub fn get_choices(&self) -> Option<Vec<Choice>> {
        match self {
            Prompt::Choice(choices) => Some(choices.clone()),
            _ => None,
        }
    }
}

/// Convenience type to indicate when a buffer of `Line` objects is being manipulated.
pub type LineBuffer = Vec<Line>;

/// Convenience type for a set of global variables.
pub type VariableSet = HashMap<String, VariableInfo>;

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Information about a global variable in the story.
pub struct VariableInfo {
    /// Whether or not the variable is constant.
    pub is_const: bool,
    /// Variable data.
    pub variable: Variable,
    /// Information about the origin of the variable in the story file or text.
    pub meta_data: MetaData,
}

impl VariableInfo {
    /// Assign a new value to the variable.
    ///
    /// Asserts that the variable is non-constant, returns an error if it is.
    pub fn assign(&mut self, variable: Variable, name: &str) -> Result<(), InklingError> {
        if self.is_const {
            Err(InklingError::AssignedToConst {
                name: name.to_string(),
            })
        } else {
            self.variable.assign(variable).map_err(|err| err.into())
        }
    }

    #[cfg(test)]
    pub fn new<T: Into<Variable>>(variable: T, line_index: usize) -> Self {
        VariableInfo {
            is_const: false,
            variable: variable.into(),
            meta_data: line_index.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigning_new_value_to_variable_info_works() {
        let mut variable_info = VariableInfo::new(Variable::Int(5), 0);

        let new = Variable::Int(10);

        assert!(variable_info.assign(new.clone(), "").is_ok());
        assert_eq!(variable_info.variable, new);
    }

    #[test]
    fn assigning_new_value_to_const_variable_info_yields_error() {
        let mut variable_info = VariableInfo::new(Variable::Int(5), 0);
        variable_info.is_const = true;

        let err = variable_info
            .assign(Variable::Int(10), "variable")
            .unwrap_err();
        let expected_err = InklingError::AssignedToConst {
            name: "variable".to_string(),
        };

        assert_eq!(format!("{:?}", err), format!("{:?}", expected_err));
    }
}
