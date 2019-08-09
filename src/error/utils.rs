//! Utilities for printing and handling errors.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Information about the origin of an item.
///
/// To be used to present errors when during parsing or runtime, allowing access to where
/// the error originated from.
pub struct MetaData {
    /// Which line in the original story the item originated from.
    pub line_index: u32,
}

#[cfg(test)]
impl From<usize> for MetaData {
    fn from(line_index: usize) -> Self {
        MetaData {
            line_index: line_index as u32,
        }
    }
}

#[cfg(test)]
impl From<()> for MetaData {
    fn from(_: ()) -> Self {
        MetaData { line_index: 0 }
    }
}

pub fn write_line_content<W: fmt::Write>(buffer: &mut W, line: &str) -> fmt::Result {
    write!(buffer, " (line was: '{}'", line)
}

pub fn write_line_information<W: fmt::Write>(buffer: &mut W, meta_data: &MetaData) -> fmt::Result {
    write!(buffer, "(line {}) ", meta_data.line_index + 1)
}
