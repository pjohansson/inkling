//! Parsing lines of `Ink` content.
//!
//! While [`InternalLine`][crate::line::InternalLine] and
//! [`InternalChoice`][crate::line::InternalChoice] are the basic data of all lines in a story,
//! the main focus of this module is the [`ParsedLineKind`][crate::line::ParsedLineKind] object.
//!
//! This is because to construct the branching tree of story content we require information
//! about which nested level every choice and gather point is found at. `ParsedLineKind` is
//! marked up with this information along with the regular internal choice and line data.
//!
//! After constructing the node tree the information about levels is discarded.
//! Thus `ParsedLineKind` is a temporary object, used only while parsing an `Ink` story.

mod alternative;
mod choice;
mod condition;
mod gather;
mod kind;
mod line;

pub(self) use choice::parse_choice;
pub(self) use condition::parse_choice_conditions;
pub(self) use gather::parse_gather;
pub use kind::{parse_line, ParsedLineKind};
pub(self) use kind::{parse_markers_and_text, split_at_divert_marker};
pub use line::{parse_chunk, parse_internal_line, LineErrorKind, LineParsingError};
