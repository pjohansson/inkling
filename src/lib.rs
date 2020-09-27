//! Partial implementation of the *Ink* markup language for game dialogue.
//!
//! Ink is a creation of [Inkle](https://www.inklestudios.com/). For more information
//! about the language, [see their website](https://www.inklestudios.com/ink/).
//!
//! # User Guide
//! A guide detailing how to use `inkling` in a more informal manner is available
//! [here](https://pjohansson.github.io/inkling/).
//!
//! # Examples
//! An example text based story player is available to run and browse the source for.
//! Run `cargo run --example player` in the source directory to try it out.
//!
//! # Features
//!
//! ## `serde_support`
//! Enable the `serde_support` feature to derive `Deserialize` and `Serialize` for all
//! required objects. If you are unfamiliar with `serde`, this corresponds to reading
//! and writing finished story files in their current state. In game terms: saving
//! and loading.
//!
//! ## `random`
//! Proper shuffle sequences using the `{~One|Two|Three}` syntax are enabled with
//! the `random` feature. This adds `rand` and `rand_chacha` as dependencies.
//! If combined with `serde_support`, the random number generator state will be
//! properly saved and restored along with the rest of the data.
//!
//! # Contributions
//! I am a complete novice at designing frameworks which will fit into larger schemes.
//! As such I have no real idea of best practices for interacting with an engine like this.
//! If you have a suggestion for how to make it easier for a user to run the processor
//! I would be very glad to hear it!
//!
//! Likewise, contributions are welcome. Please open an issue on
//! [Github](https://github.com/pjohansson/inkling) to discuss improvements or submit
//! a pull request.

mod consts;
mod derives;
pub mod error;
mod follow;
mod knot;
mod line;
pub mod log;
mod node;
mod process;
mod story;

pub use error::InklingError;
pub use line::Variable;
pub use log::Logger;
pub use story::{read_story_from_string, utils, Choice, Line, LineBuffer, Location, Prompt, Story};
