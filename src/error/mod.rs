//! Errors from creating or walking through stories.

mod error;
mod parse;

pub use error::{InklingError, InvalidAddressError};
pub use parse::ParseError;

pub(crate) use error::{
    BadGraphKind, IncorrectNodeStackKind, InternalError, NodeItemKind, StackError, WhichIndex,
};
pub(crate) use parse::{KnotError, KnotNameError, LineError};
