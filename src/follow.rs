use crate::{
    error::InklingError,
    line::{InternalChoice, InternalLine},
};

pub type FollowResult = Result<Next, InklingError>;

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceInfo {
    pub num_visited: u32,
    pub choice_data: InternalChoice,
}

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
