//! Partial implementation of the *Ink* markup language for game dialogue.
//!
//! Ink is a creation of [Inkle](https://www.inklestudios.com/). For more information
//! about the language, [see their website](https://www.inklestudios.com/ink/).

mod consts;
pub mod error;
mod follow;
mod knot;
mod line;
mod node;
mod story;

pub use story::{
    copy_lines_into_string, read_story_from_string, Choice, Line, LineBuffer, Prompt, Story,
};
