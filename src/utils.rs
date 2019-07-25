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
