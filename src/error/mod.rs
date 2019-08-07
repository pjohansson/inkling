//! Errors from creating or walking through stories.

#[macro_use]
mod error;
pub(crate) mod parse;

pub(crate) use error::IncorrectNodeStackError;
pub use error::{InklingError, VariableError, VariableErrorKind};
pub use parse::ParseError;

pub(crate) use error::{InternalError, ProcessError, ProcessErrorKind, StackError};
pub(crate) use parse::{
    ConditionError, ConditionErrorKind, InvalidAddressError, KnotError, KnotNameError, LineError,
    LineErrorKind,
};
