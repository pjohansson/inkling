//! Errors from creating or walking through stories.

#[macro_use]
mod error;
pub mod parse;
pub(self) mod utils;

pub(crate) use error::IncorrectNodeStackError;
pub use error::{InklingError, VariableError, VariableErrorKind};
pub use parse::ReadError;

pub(crate) use error::{InternalError, ProcessError, ProcessErrorKind, StackError};
pub(crate) use parse::{
    ConditionError, ConditionErrorKind, InvalidAddressError, KnotErrorKind, KnotNameError,
    LineError, LineErrorKind,
};
