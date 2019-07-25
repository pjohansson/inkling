//! Structures representing a complete `Ink` story.

mod address;
mod parse;
mod process;
mod story;
mod utils;

pub use address::Address;
pub use story::{read_story_from_string, Choice, Knots, Line, LineBuffer, Prompt, Story};
pub use utils::copy_lines_into_string;
