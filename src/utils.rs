//! Utilities and derives used elsewhere in the library.

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serde_support")]
use std::cmp::Ordering;

#[cfg(feature = "serde_support")]
#[derive(Deserialize, Serialize)]
#[serde(remote = "Ordering")]
/// Remote type to derive `Deserialize` and `Serialize` for `Ordering`.
pub enum OrderingDerive {
    Equal,
    Less,
    Greater,
}

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
