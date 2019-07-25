use crate::{
    error::InklingError,
    line::{InternalChoice, InternalLine},
};

pub type FollowResult = Result<Next, InklingError>;

pub type LineDataBuffer = Vec<InternalLine>;

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum Next {
    /// Finished with the current node or story.
    Done,
    /// Divert to a new knot with the given name.
    Divert(String),
    /// Choice for the user.
    ChoiceSet(Vec<ChoiceInfo>),
}

#[derive(Clone, Debug, PartialEq)]
/// Information about a branching choice encountered in the story.
pub struct ChoiceInfo {
    /// Number of times that the branching node (not the choice itself) has been seen.
    pub num_visited: u32,
    /// Choice data to process before presenting to the user.
    pub choice_data: InternalChoice,
}