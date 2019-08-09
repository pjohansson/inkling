#[macro_use]
pub(crate) mod inkling;
pub(crate) mod internal;
pub mod variable;

pub use inkling::InklingError;
pub use internal::*;
pub use variable::*;
