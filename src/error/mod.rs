//! Errors from creating or walking through stories.

#[macro_use]
pub(crate) mod utils;
pub mod parse;
pub(crate) mod runtime;

pub use parse::ReadError;
pub use runtime::{variable, InklingError, InternalError};
pub use utils::MetaData;

pub(crate) use parse::{
    ConditionError, ConditionErrorKind, InvalidAddressError, KnotErrorKind, KnotNameError,
    LineError, LineErrorKind,
};
pub(crate) use runtime::{
    internal::{IncorrectNodeStackError, ProcessError, ProcessErrorKind, StackError},
    variable::{VariableError, VariableErrorKind},
};
