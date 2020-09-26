//! Utilities for inspecting warnings and other non-fatal errors.

mod logger;
mod message;

pub use logger::Logger;
pub use message::{LogMessage, MessageKind, Warning};
