//! Errors from creating or walking through stories.

#[macro_use]
mod error;
mod parse;

pub(crate) use error::IncorrectNodeStackError;
pub use error::InklingError;
pub use parse::ParseError;

pub(crate) use error::{
    InternalError, ProcessError, ProcessErrorKind, StackError, VariableError, VariableErrorKind,
};
pub(crate) use parse::{
    BadCondition, BadConditionKind, InvalidAddressError, KnotError, KnotNameError, LineErrorKind,
    LineParsingError,
};
