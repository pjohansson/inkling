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

/// Write meta data information for a line or piece of content in a story.
pub(crate) fn write_line_information<W: fmt::Write>(
    buffer: &mut W,
    meta_data: &MetaData,
) -> fmt::Result {
    write!(buffer, "(line {}) ", meta_data.line_index + 1)
}

/// Wrapper to implement From for variants when the variant is simply encapsulated
/// in the enum.
///
/// # Example
/// Running
/// ```
/// impl_from_error[
///     MyError,
///     [Variant, ErrorData]
/// ];
/// ```
/// is identical to running
/// ```
/// impl From<ErrorData> for MyError {
///     from(err: ErrorData) -> Self {
///         Self::Variant(err)
///     }
/// }
/// ```
/// The macro can also implement several variants at once:
/// ```
/// impl_from_error[
///     MyError,
///     [Variant1, ErrorData1],
///     [Variant2, ErrorData2]
/// ];
/// ```
macro_rules! impl_from_error {
    ($for_type:ident; $([$variant:ident, $from_type:ident]),+) => {
        $(
            impl From<$from_type> for $for_type {
                fn from(err: $from_type) -> Self {
                    $for_type::$variant(err)
                }
            }
        )*
    }
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
