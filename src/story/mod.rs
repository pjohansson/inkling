mod parse;
mod process;
mod story;
mod utils;

pub use utils::copy_lines_into_string;
pub use story::{read_story_from_string, Line, LineBuffer, Story, StoryAction};
