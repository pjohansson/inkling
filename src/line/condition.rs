//! Conditions for displaying choices, lines or other content.

use crate::{
    error::InvalidAddressError,
    story::{Address, Knots, ValidateAddresses},
};

use std::{cmp::Ordering, error::Error};

#[cfg(feature = "serde_support")]
use crate::utils::OrderingDerive;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Condition for displaying some content or choice in the story.
pub struct Condition {
    /// First condition to evaluate.
    pub kind: ConditionKind,
    /// Ordered set of `and`/`or` conditions to compare the first condition to.
    pub items: Vec<AndOr>,
}

impl Condition {
    /// Evaluate the condition with the given evaluator closure.
    pub fn evaluate<F, E>(&self, evaluator: F) -> Result<bool, E>
    where
        F: Fn(&ConditionKind) -> Result<bool, E>,
        E: Error,
    {
        self.items
            .iter()
            .fold(evaluator(&self.kind), |acc, next_condition| {
                acc.and_then(|current| match next_condition {
                    AndOr::And(ref condition) => evaluator(condition).map(|next| current && next),
                    AndOr::Or(ref condition) => evaluator(condition).map(|next| current || next),
                })
            })
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// ConditionKind to show choice (or maybe part of line, in the future)
pub enum ConditionKind {
    /// Condition is `true`.
    True,
    /// Condition is `false`.
    False,
    /// Use a knot (or maybe other string-like variable) to check whether its value
    /// compares to the set condition.
    NumVisits {
        address: Address,
        rhs_value: i32,
        #[cfg_attr(feature = "serde_support", serde(with = "OrderingDerive"))]
        ordering: Ordering,
        not: bool, // negation of the condition, ie. !(condition)
    },
}

pub struct ConditionBuilder {
    kind: ConditionKind,
    items: Vec<AndOr>,
}

impl ConditionBuilder {
    pub fn with_kind(kind: &ConditionKind) -> Self {
        ConditionBuilder {
            kind: kind.clone(),
            items: Vec::new(),
        }
    }

    pub fn build(self) -> Condition {
        Condition {
            kind: self.kind,
            items: self.items,
        }
    }

    pub fn and(&mut self, kind: &ConditionKind) {
        self.items.push(AndOr::And(kind.clone()));
    }

    pub fn or(&mut self, kind: &ConditionKind) {
        self.items.push(AndOr::Or(kind.clone()));
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Container for `and`/`or` variants of conditions.
pub enum AndOr {
    And(ConditionKind),
    Or(ConditionKind),
}

impl ValidateAddresses for Condition {
    fn validate(
        &mut self,
        current_address: &Address,
        knots: &Knots,
    ) -> Result<(), InvalidAddressError> {
        unimplemented!();
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        unimplemented!();
    }
}

impl ValidateAddresses for ConditionKind {
    fn validate(
        &mut self,
        current_address: &Address,
        knots: &Knots,
    ) -> Result<(), InvalidAddressError> {
        match self {
            ConditionKind::NumVisits {
                ref mut address, ..
            } => address.validate(current_address, knots),
            ConditionKind::True | ConditionKind::False => Ok(()),
        }
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        match self {
            ConditionKind::NumVisits { address, .. } => address.all_addresses_are_valid(),
            ConditionKind::True | ConditionKind::False => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt;

    use ConditionKind::{False, True};

    impl Condition {
        pub fn and(mut self, kind: ConditionKind) -> Self {
            self.items.push(AndOr::And(kind));
            self
        }

        pub fn or(mut self, kind: ConditionKind) -> Self {
            self.items.push(AndOr::Or(kind));
            self
        }
    }

    impl From<ConditionKind> for Condition {
        fn from(kind: ConditionKind) -> Self {
            Condition {
                kind,
                items: Vec::new(),
            }
        }
    }

    #[derive(Debug)]
    struct MockError;

    impl fmt::Display for MockError {
        fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
            unreachable!();
        }
    }

    impl Error for MockError {}

    #[test]
    fn condition_links_from_left_to_right() {
        let f = |kind: &ConditionKind| match kind {
            ConditionKind::True => Ok(true),
            ConditionKind::False => Ok(false),
            _ => Err(MockError),
        };

        assert!(Condition::from(True).evaluate(f).unwrap());
        assert!(!Condition::from(False).evaluate(f).unwrap());

        assert!(Condition::from(True).and(True).evaluate(f).unwrap());
        assert!(!Condition::from(True).and(False).evaluate(f).unwrap());

        assert!(Condition::from(False)
            .and(False)
            .or(True)
            .evaluate(f)
            .unwrap());
        assert!(!Condition::from(False)
            .and(False)
            .or(True)
            .and(False)
            .evaluate(f)
            .unwrap());
    }

    #[test]
    fn evaluator_function_can_use_local_variables() {
        let needle = ConditionKind::False;

        let f = |kind: &ConditionKind| -> Result<bool, MockError> { Ok(kind == &needle) };

        assert!(Condition::from(False).evaluate(f).unwrap());
        assert!(!Condition::from(True).evaluate(f).unwrap());
    }
}
