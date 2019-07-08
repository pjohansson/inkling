use crate::{
    error::{FollowError, InternalError},
    knot::Knot,
    line::{Choice, Line},
};

use std::collections::HashMap;

pub type FollowResult = Result<Next, FollowError>;

#[derive(Debug)]
pub struct Story {
    knots: HashMap<String, Knot>,
    stack: Vec<String>,
}

impl Story {
    pub fn follow_knot(&mut self, buffer: &mut LineBuffer) -> FollowResult {
        self.call_on_knot(|knot| knot.follow(buffer))
    }

    pub fn follow_knot_with_choice(
        &mut self,
        choice_index: usize,
        buffer: &mut LineBuffer,
    ) -> FollowResult {
        self.call_on_knot(|knot| knot.follow_with_choice(choice_index, buffer))
    }

    fn call_on_knot<F>(&mut self, f: F) -> FollowResult
    where
        F: FnOnce(&mut Knot) -> FollowResult,
    {
        let knot_name = self.stack.last().unwrap();

        self.knots
            .get_mut(knot_name)
            .ok_or(
                InternalError::UnknownKnot {
                    name: knot_name.clone(),
                }
                .into(),
            )
            .and_then(f)
    }
}

pub type LineBuffer = Vec<Line>;

#[derive(Clone, Debug, PartialEq)]
/// What action that is prompted by following a story.
pub enum Next {
    /// Finished with the current node or story.
    Done,
    /// Divert to a new knot with the given name.
    Divert(String),
    /// Choice for the user.
    ChoiceSet(Vec<Choice>),
}

#[cfg(test)]
mod tests {
    use super::*;

}
