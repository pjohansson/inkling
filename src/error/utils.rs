//! Utilities for printing and handling errors.

use std::fmt;

use crate::utils::MetaData;

pub fn write_line_content<W: fmt::Write>(buffer: &mut W, line: &str) -> fmt::Result {
    write!(buffer, " (line was: '{}'", line)
}

pub fn write_line_information<W: fmt::Write>(buffer: &mut W, meta_data: &MetaData) -> fmt::Result {
    write!(buffer, "(line {}) ", meta_data.line_index + 1)
}
