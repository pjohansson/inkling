//! Utilities for inspecting warnings and other non-fatal errors.
//!
//! The main object for logging items is [`Logger`][crate::log::Logger], which stores warnings
//! and to-do comments from parsing a script with `inkling`. Its messages can be iterated
//! over and inspected, or printed to string buffers or files using regular formatting tools.
//! It is recommended that you inspect this log when running your software, to ensure that any
//! unexpected behavior in the script is understood.

mod logger;
mod message;

pub use logger::Logger;
pub use message::{LogMessage, MessageKind, Warning};
