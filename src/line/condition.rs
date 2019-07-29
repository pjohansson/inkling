//! Conditions for displaying choices, lines or other content.

use crate::{
    error::InvalidAddressError,
    story::{Address, Knots, ValidateAddresses},
};

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
        name: Address,
        rhs_value: i32,
        #[cfg_attr(feature = "serde_support", serde(with = "OrderingDerive"))]
        ordering: Ordering,
        not: bool, // negation of the condition, ie. !(condition)
    },
}

impl ValidateAddresses for Condition {
    fn validate(&mut self, current_address: &Address, knots: &Knots) -> Result<(), InvalidAddressError> {
        match self {
            Condition::NumVisits { ref mut name, .. } => name.validate(current_address, knots),
        }
    }

    fn all_addresses_are_valid(&self) -> bool {
        match self {
            Condition::NumVisits { name, .. } => name.all_addresses_are_valid(),
        }
    }
}
