//! Utilities for inspecting warnings and other non-fatal errors.
//!
//! The main object for logging items is [`Logger`][crate::log::Logger], which stores warnings
//! and to-do comments from parsing a script with `inkling`. Its messages can be iterated
//! over and inspected, or printed to string buffers or files using regular formatting tools.
//! It is recommended that you inspect this log when running your software, to ensure that any
//! unexpected behavior in the script is understood.
//!
//! # Example
//! ## Printing log to standard error
//! ```
//! # use inkling::read_story_from_string;
//! # let content = "Empty story.";
//! let story = read_story_from_string(content).unwrap();
//!
//! for message in story.get_log().iter() {
//!     eprintln!("{}", message);
//! }
//! ```
//!
//! ## To-do comments
//! ```
//! # use inkling::{log::MessageKind, read_story_from_string};
//! let content = "\
//! === arrival_at_château ===
//! TODO: Finish initial scene.
//! By 11 PM I had arrived at the mansion.
//! ";
//!
//! let story = read_story_from_string(content).unwrap();
//!
//! let log = story.get_log();
//! assert_eq!(log.todo_comments.len(), 1);
//!
//! for comment in log.todo_comments.iter() {
//!     eprintln!("{}", comment);
//! }
//! ```
//!

mod logger;
mod message;

pub use logger::Logger;
pub use message::{LogMessage, MessageKind, Warning};
