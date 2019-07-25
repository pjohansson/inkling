//! Conditions for displaying choices, lines or other content.

use std::cmp::Ordering;

#[cfg(feature = "serde_support")]
use crate::utils::OrderingDerive;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Condition to show choice (or maybe part of line, in the future)
pub enum Condition {
    /// Use a knot (or maybe other string-like variable) to check whether its value
    /// compares to the set condition.
    NumVisits {
        name: String,
        rhs_value: i32,
        #[cfg_attr(feature = "serde_support", serde(with = "OrderingDerive"))]
        ordering: Ordering,
        not: bool, // negation of the condition, ie. !(condition)
    },
}
