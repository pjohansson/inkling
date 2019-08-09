//! Errors from creating or walking through stories.

#[macro_use]
pub(crate) mod utils;
pub mod parse;
pub(crate) mod runtime;

pub use parse::ReadError;
pub use runtime::{variable, InklingError, InternalError};
pub use utils::MetaData;
