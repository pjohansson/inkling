//! Utilities for printing and handling errors.

use std::{fmt, io};

use crate::utils::MetaData;

pub fn print_line_information(meta_data: &MetaData) -> String {
    format!("(line {})", meta_data.line_index + 1)
}

pub fn write_line_information<W: fmt::Write>(buffer: &mut W, meta_data: &MetaData) -> fmt::Result {
    write!(buffer, "(line {}) ", meta_data.line_index + 1)
}
