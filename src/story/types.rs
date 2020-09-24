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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Knot and (possible) stitch location in the story.
///
/// Can be used to move to new locations with `Story::move_to`.
///
/// Implements `From<&str>` for strings. Strings are parsed like `Ink` addresses
/// in `knot.stitch` format.
///
/// # Examples
///
/// ## Move to a new location
/// ```
/// # use inkling::{read_story_from_string, Location};
/// let content = "\
/// == 24th_island_sequence
/// Island sequence 24 is ending.
///
/// == 25th_island_sequence
/// Perfect 25 is awaiting.
/// ";
///
/// let mut story = read_story_from_string(content).unwrap();
///
/// let twenty_fifth = Location {
///     knot: "25th_island_sequence".to_string(),
///     stitch: None,
/// };
///
/// story.move_to(&twenty_fifth).unwrap();
/// assert_eq!(&story.get_current_location(), &twenty_fifth);
/// ```
///
/// ## Parsing from strings
/// ```
/// # use inkling::Location;
/// assert_eq!(
///     Location::from("25th_island_sequence"),
///     Location {
///         knot: "25th_island_sequence".to_string(),
///         stitch: None,
///     }
/// );
///
/// assert_eq!(
///     Location::from("24th_island_sequence.pyramids"),
///     Location {
///         knot: "24th_island_sequence".to_string(),
///         stitch: Some("pyramids".to_string()),
///     }
/// );
/// ```
pub struct Location {
    pub knot: String,
    pub stitch: Option<String>,
}

impl From<&str> for Location {
    fn from(address: &str) -> Self {
        if let Some(i) = address.find('.') {
            let (knot, stitch) = address.split_at(i);
            Location::with_stitch(knot, stitch.get(1..).unwrap())
        } else {
            Location::new(address, None)
        }
    }
}

impl Location {
    /// Create a `Location` with a knot and possible stitch address.
    ///
    /// # Examples
    /// ```
    /// # use inkling::Location;
    /// assert_eq!(
    ///     Location::new("gesichts_apartment", None),
    ///     Location {
    ///         knot: "gesichts_apartment".to_string(),
    ///         stitch: None,
    ///     }
    /// );
    ///
    /// assert_eq!(
    ///     Location::new("mirandas_den", Some("dream")),
    ///     Location {
    ///         knot: "mirandas_den".to_string(),
    ///         stitch: Some("dream".to_string()),
    ///     }
    /// );
    /// ```
    pub fn new<S: ToString>(knot: S, stitch: Option<S>) -> Self {
        Location {
            knot: knot.to_string(),
            stitch: stitch.map(|s| s.to_string()),
        }
    }

    /// Create a `Location` with knot and stitch address.
    ///
    /// # Examples
    /// ```
    /// # use inkling::Location;
    /// assert_eq!(
    ///     Location::with_stitch("mirandas_den", "dream"),
    ///     Location {
    ///         knot: "mirandas_den".to_string(),
    ///         stitch: Some("dream".to_string()),
    ///     }
    /// );
    /// ```
    pub fn with_stitch<S: ToString>(knot: S, stitch: S) -> Self {
        Location {
            knot: knot.to_string(),
            stitch: Some(stitch.to_string()),
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

    #[test]
    fn location_from_string_sets_knot_if_no_periods_are_involved() {
        assert_eq!(
            Location::from("knot_address"),
            Location::new("knot_address", None),
        );
    }

    #[test]
    fn location_from_string_splits_knot_and_stitch_at_period_if_present() {
        assert_eq!(
            Location::from("knot_address.stitch_address"),
            Location::with_stitch("knot_address", "stitch_address"),
        );
    }

    #[test]
    fn location_from_string_splits_at_first_period_if_multiple() {
        assert_eq!(
            Location::from("knot_address.stitch_address.second"),
            Location::with_stitch("knot_address", "stitch_address.second"),
        );
    }

    #[test]
    fn location_from_string_returns_address_if_nothing_after_period() {
        assert_eq!(
            Location::from("knot_address."),
            Location::with_stitch("knot_address", ""),
        );
    }
}
