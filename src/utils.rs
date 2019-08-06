//! Utilities and derives used elsewhere in the library.

#[cfg(feature = "serde_support")]
use serde::*;
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
/// Information about the origin of an item.
///
/// To be used to present errors when during parsing or runtime, allowing access to where
/// the error originated from.
pub struct MetaData {
    /// Which line in the original story the item originated from.
    pub line_index: usize,
}
