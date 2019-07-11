use crate::{
    error::InklingError,
    line::{ChoiceData, LineData},
};

pub type FollowResult = Result<Next, InklingError>;
pub type LineDataBuffer = Vec<LineData>;

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum Next {
    /// Finished with the current node or story.
    Done,
    /// Divert to a new knot with the given name.
    Divert(String),
    /// Choice for the user.
    ChoiceSet(Vec<ChoiceData>),
}
