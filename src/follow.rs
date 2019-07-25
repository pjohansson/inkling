use crate::{
    error::InklingError,
    line::{FullChoice, FullLine},
};

pub type FollowResult = Result<Next, InklingError>;

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceExtra {
    pub num_visited: u32,
    pub choice_data: FullChoice,
}

pub type LineDataBuffer = Vec<FullLine>;

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum Next {
    /// Finished with the current node or story.
    Done,
    /// Divert to a new knot with the given name.
    Divert(String),
    /// Choice for the user.
    ChoiceSet(Vec<ChoiceExtra>),
}
