//! Errors from creating or walking through stories.

mod error;
mod parse;

pub use error::{IncorrectNodeStackError, InklingError, InvalidAddressError};
pub use parse::ParseError;

pub(crate) use error::{InternalError, StackError};
pub(crate) use parse::{KnotError, KnotNameError, LineError};
