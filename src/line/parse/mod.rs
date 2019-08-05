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
mod expression;
mod gather;
mod kind;
mod line;
mod utils;
mod variable;

pub(self) use alternative::parse_alternative;
pub(self) use choice::parse_choice;
pub(self) use condition::{parse_choice_condition, parse_line_condition};
pub(self) use expression::parse_expression;
pub(self) use gather::parse_gather;
pub use kind::{parse_line, ParsedLineKind};
pub(self) use kind::{parse_markers_and_text, split_at_divert_marker};
pub use line::{parse_chunk, parse_internal_line, validate_address};
pub(self) use utils::{
    split_line_at_separator_braces, split_line_at_separator_parenthesis,
    split_line_into_groups_braces, LinePart,
};
pub use variable::parse_variable;
