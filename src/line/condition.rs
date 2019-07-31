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

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Condition for displaying some content or choice in the story.
struct Condition {
    /// First condition to evaluate.
    kind: ConditionKind,
    /// Ordered set of `and`/`or` conditions to compare the first condition to.
    items: Vec<AndOr>,
}

impl Condition {
    /// Evaluate the condition with the given evaluator closure.
    fn evaluate<F>(&self, evaluator: F) -> bool where F: Fn(&ConditionKind) -> bool {
        self.items.iter().fold(evaluator(&self.kind), |acc, next| {
            match next {
                AndOr::And(ref condition) => acc && evaluator(condition),
                AndOr::Or(ref condition) => acc || evaluator(condition),
            }
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Container for `and`/`or` variants of conditions.
enum AndOr {
    And(ConditionKind),
    Or(ConditionKind),
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

    use ConditionKind::{True, False};

    impl Condition {
        fn and(mut self, kind: ConditionKind) -> Self {
            self.items.push(AndOr::And(kind));
            self
        }

        fn or(mut self, kind: ConditionKind) -> Self {
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

    #[test]
    fn condition_links_from_left_to_right() {
        let f = |kind: &ConditionKind| match kind {
            ConditionKind::True => true,
            ConditionKind::False => false,
            _ => unreachable!(),
        };

        assert!(Condition::from(True).evaluate(f));
        assert!(!Condition::from(False).evaluate(f));

        assert!(Condition::from(True).and(True).evaluate(f));
        assert!(!Condition::from(True).and(False).evaluate(f));

        assert!(Condition::from(False).and(False).or(True).evaluate(f));
        assert!(!Condition::from(False).and(False).or(True).and(False).evaluate(f));
    }

    #[test]
    fn evaluator_function_can_use_local_variables() {
        let needle = ConditionKind::False;

        let f = |kind: &ConditionKind| kind == &needle;

        assert!(Condition::from(False).evaluate(f));
        assert!(!Condition::from(True).evaluate(f));
    }
}
