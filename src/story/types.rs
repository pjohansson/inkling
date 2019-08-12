//! Data types of a story.

use crate::{error::utils::MetaData, line::Variable};

use std::collections::HashMap;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Debug)]
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
/// story.start().unwrap();
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
    /// story.start().unwrap();
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

pub type VariableSet = HashMap<String, VariableInfo>;

#[derive(Clone, Debug)]
pub struct VariableInfo {
    pub variable: Variable,
    pub meta_data: MetaData,
}

#[cfg(test)]
impl VariableInfo {
    pub fn new<T: Into<Variable>>(variable: T, line_index: usize) -> Self {
        VariableInfo {
            variable: variable.into(),
            meta_data: line_index.into(),
        }
    }
}
