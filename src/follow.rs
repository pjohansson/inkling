//! Results and data that is used or encountered when following, or walking through, a story.

use crate::{
    error::InklingError,
    knot::Address,
    line::InternalChoice,
    story::{rng::StoryRng, types::VariableSet},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// Convenience type for a result of the encountered event and main error type.
pub type FollowResult = Result<EncounteredEvent, InklingError>;

/// Buffer of text and associated content that will be constructed for every seen line.
pub type LineDataBuffer = Vec<LineText>;

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum EncounteredEvent {
    /// Choice for the user.
    BranchingChoice(Vec<ChoiceInfo>),
    /// Divert to a new knot with the given name.
    Divert(Address),
    /// Finished with the current node or story.
    Done,
}

#[derive(Clone, Debug, PartialEq)]
/// Information about a branching choice encountered in the story.
pub struct ChoiceInfo {
    /// Number of times that the branching node (not the choice itself) has been seen.
    pub num_visited: u32,
    /// Choice data to process before presenting to the user.
    pub choice_data: InternalChoice,
}

impl ChoiceInfo {
    /// Create the information container from given data.
    pub fn from_choice(choice: &InternalChoice, num_visited: u32) -> Self {
        ChoiceInfo {
            num_visited,
            choice_data: choice.clone(),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Data used during a follow through knots and nodes.
pub struct FollowData {
    /// Number of times a knot and stitch address has been visited.
    pub knot_visit_counts: HashMap<String, HashMap<String, u32>>,
    /// Global variables in story.
    pub variables: VariableSet,
    /// Random number generator
    pub rng: StoryRng,
}

#[derive(Clone, Debug, PartialEq)]
/// Processed text from a full line.
///
/// This is the result from calling [`process_line`][crate::process::process_line] on a single
/// [`InternalLine`][crate::line::InternalLine] object and filling in the remaining
/// information. It is one possible result from that object since the `Process` trait
/// can handle variations depending on variables or alternatives.
pub struct LineText {
    /// Processed text.
    ///
    /// The result while not yet have been trimmed of extraneous whitespace between
    /// words or lines.
    pub text: String,
    /// Whether or not the line glues to the next line.
    pub glue_begin: bool,
    /// Whether or not the line glues to the previous line.
    pub glue_end: bool,
    /// Tags associated with the line.
    pub tags: Vec<String>,
}

#[cfg(test)]
/// Constructing struct for `LineText` objects.
///
/// Used for test creation of prepared `LineDataBuffer` objects.
pub struct LineTextBuilder {
    pub text: String,
    pub glue_begin: bool,
    pub glue_end: bool,
    pub tags: Vec<String>,
}

#[cfg(test)]
impl LineTextBuilder {
    pub fn from_string(content: &str) -> Self {
        LineTextBuilder {
            text: content.to_string(),
            glue_begin: false,
            glue_end: false,
            tags: Vec::new(),
        }
    }

    pub fn build(self) -> LineText {
        LineText {
            text: self.text,
            glue_begin: self.glue_begin,
            glue_end: self.glue_end,
            tags: self.tags,
        }
    }

    pub fn with_glue_begin(mut self) -> Self {
        self.glue_begin = true;
        self
    }

    pub fn with_glue_end(mut self) -> Self {
        self.glue_end = true;
        self
    }

    pub fn with_tags(mut self, tags: &[String]) -> Self {
        self.tags = tags.to_vec();
        self
    }
}

#[cfg(test)]
/// Builder for `FollowData` during tests
pub struct FollowDataBuilder {
    knot_visit_counts: HashMap<String, HashMap<String, u32>>,
    variables: VariableSet,
    rng: StoryRng,
}

#[cfg(test)]
impl FollowDataBuilder {
    pub fn new() -> Self {
        FollowDataBuilder {
            knot_visit_counts: HashMap::new(),
            variables: VariableSet::new(),
            rng: StoryRng::default(),
        }
    }

    pub fn with_knots(mut self, knot_visit_counts: HashMap<String, HashMap<String, u32>>) -> Self {
        self.knot_visit_counts = knot_visit_counts;
        self
    }

    pub fn with_variables(mut self, variables: VariableSet) -> Self {
        self.variables = variables;
        self
    }

    pub fn with_rng(mut self, rng: StoryRng) -> Self {
        self.rng = rng;
        self
    }

    pub fn build(self) -> FollowData {
        FollowData {
            knot_visit_counts: self.knot_visit_counts,
            variables: self.variables,
            rng: self.rng,
        }
    }
}
