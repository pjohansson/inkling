//! Errors from creating or walking through stories.

mod error;
mod parse;

pub use error::InklingError;
pub use parse::ParseError;

pub(crate) use error::{BadGraphKind, IncorrectNodeStackKind, InvalidAddressError, NodeItemKind, StackError, WhichIndex};
pub(crate) use parse::{KnotError, KnotNameError, LineError};
