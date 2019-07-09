mod parse;
mod process;
mod story;

pub use process::copy_lines_into_string;
pub use story::{read_story_from_string, Line, LineBuffer, Story, StoryAction};
