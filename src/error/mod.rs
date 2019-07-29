//! Errors from creating or walking through stories.

#[macro_use]
mod error;
mod parse;

pub(crate) use error::IncorrectNodeStackError;
pub use error::{InklingError, InvalidAddressError};
pub use parse::ParseError;

pub(crate) use error::{InternalError, StackError};
pub(crate) use parse::{KnotError, KnotNameError, LineErrorKind, LineParsingError};
