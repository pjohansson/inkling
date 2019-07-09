//! Errors from creating or walking through stories.

mod follow;
mod parse;

pub use follow::{FollowError, InternalError};
pub use parse::ParseError;

pub(crate) use follow::{BadGraphKind, IncorrectStackKind, NodeItemKind, WhichIndex};
pub(crate) use parse::{KnotError, LineError};
