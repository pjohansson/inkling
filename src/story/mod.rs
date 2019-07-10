mod parse;
mod process;
mod story;
mod utils;

pub use story::{read_story_from_string, Choice, Line, LineBuffer, Story, StoryAction};
pub use utils::copy_lines_into_string;
