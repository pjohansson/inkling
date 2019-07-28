//! Errors from creating or walking through stories.

#[macro_use]
mod error;
mod parse;

pub use error::{InklingError, InvalidAddressError};
pub(crate) use error::IncorrectNodeStackError;
pub use parse::ParseError;

pub(crate) use error::{InternalError, StackError};
pub(crate) use parse::{KnotError, KnotNameError, LineErrorKind, LineParsingError};
