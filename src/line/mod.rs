//! Structures representing lines of `Ink` story content.
//!
//! There are two basic types of line content in a story: lines that will be processed
//! as the story is followed and choices which represent branching points that the user
//! has to select from.
//!
//! All line content is contained in the [`InternalLine`][crate::line::InternalLine]
//! structure. This represents a single line from an `Ink` story file. It is split
//! into smaller [`LineChunk`][crate::line::LineChunk] objects, each of which will
//! be processed in turn when the line is encountered. They can be nested with alternatives,
//! conditionals and diverts which will be selected from at runtime. A line
//! may contain internal parts which are only followed during certain conditions,
//! while the rest of the line may be unaffected or always present.
//!
//! Choices are represented by the [`InternalChoice`][crate::line::InternalChoice] object.
//! This contains different variants of text to be shown to the user and once a choice
//! is made and can have conditions for when they are presented at all.
//!
//! Tying the story processor together is the [`Process`][crate::line::Process] trait
//! which is implemented on constituent parts of lines. This makes nesting into lines
//! possible.

mod alternative;
mod choice;
mod condition;
mod line;
pub(crate) mod parse;

pub(crate) use alternative::{Alternative, AlternativeBuilder, AlternativeKind};
pub(crate) use choice::{InternalChoice, InternalChoiceBuilder};
pub(crate) use condition::{
    Condition, ConditionBuilder, ConditionItem, ConditionKind, StoryCondition,
};
#[cfg(test)]
pub(crate) use line::builders::LineChunkBuilder;
pub(crate) use line::{builders::InternalLineBuilder, Content, InternalLine, LineChunk};
pub(crate) use parse::{parse_line, ParsedLineKind};
