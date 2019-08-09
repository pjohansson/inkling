#[macro_use]
pub(crate) mod error;
pub(crate) mod internal;
pub mod variable;

pub use error::InklingError;
pub use internal::InternalError;
