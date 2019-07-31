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
    pub root: ConditionKind,
    /// Ordered set of `and`/`or` conditions to compare the first condition to.
    pub items: Vec<AndOr>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Base item in a condition.
///
/// Will evaluate to a single `true` or `false` but may have to evaluate a group
/// of conditions.
pub enum ConditionKind {
    /// Always `true`.
    True,
    /// Always `false`.
    False,
    /// Nested `Condition` which has to be evaluated as a group.
    Nested(Box<Condition>),
    /// Single condition to evaluate.
    Single(StoryCondition),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Condition to show some content in a story.
pub enum StoryCondition {
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
/// Container for `and`/`or` variants of conditions to evaluate in a list.
pub enum AndOr {
    And(ConditionKind),
    Or(ConditionKind),
}

impl Condition {
    /// Evaluate the condition with the given evaluator closure.
    pub fn evaluate<F, E>(&self, evaluator: &F) -> Result<bool, E>
    where
        F: Fn(&StoryCondition) -> Result<bool, E>,
        E: Error,
    {
        self.items
            .iter()
            .fold(inner_eval(&self.root, evaluator), |acc, next_condition| {
                acc.and_then(|current| match next_condition {
                    AndOr::And(ref item) => inner_eval(item, evaluator).map(|next| current && next),
                    AndOr::Or(ref item) => inner_eval(item, evaluator).map(|next| current || next),
                })
            })
    }
}

/// Match against and evaluate the items.
fn inner_eval<F, E>(item: &ConditionKind, evaluator: &F) -> Result<bool, E>
where
    F: Fn(&StoryCondition) -> Result<bool, E>,
    E: Error,
{
    match item {
        ConditionKind::True => Ok(true),
        ConditionKind::False => Ok(false),
        ConditionKind::Nested(condition) => condition.evaluate(evaluator),
        ConditionKind::Single(kind) => evaluator(kind),
    }
}

/// Constructor struct for `Condition`.
pub struct ConditionBuilder {
    root: ConditionKind,
    items: Vec<AndOr>,
}

impl ConditionBuilder {
    /// Create the constructor with a condition kind.
    pub fn from_item(item: &ConditionKind) -> Self {
        ConditionBuilder {
            root: item.clone(),
            items: Vec::new(),
        }
    }

    /// Finalize the `Condition` and return it.
    pub fn build(self) -> Condition {
        Condition {
            root: self.root,
            items: self.items,
        }
    }

    /// Add an `and` item to the condition list.
    pub fn and(&mut self, item: &ConditionKind) {
        self.items.push(AndOr::And(item.clone()));
    }

    /// Add an `or` item to the condition list.
    pub fn or(&mut self, item: &ConditionKind) {
        self.items.push(AndOr::Or(item.clone()));
    }
}

impl From<StoryCondition> for ConditionKind {
    fn from(kind: StoryCondition) -> Self {
        ConditionKind::Single(kind)
    }
}

impl From<&StoryCondition> for ConditionKind {
    fn from(kind: &StoryCondition) -> Self {
        ConditionKind::Single(kind.clone())
    }
}

impl ValidateAddresses for Condition {
    fn validate(
        &mut self,
        current_address: &Address,
        knots: &Knots,
    ) -> Result<(), InvalidAddressError> {
        self.root.validate(current_address, knots)?;

        self.items
            .iter_mut()
            .map(|item| match item {
                AndOr::And(item) | AndOr::Or(item) => item.validate(current_address, knots),
            })
            .collect()
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
            ConditionKind::True | ConditionKind::False => Ok(()),
            ConditionKind::Nested(condition) => condition.validate(current_address, knots),
            ConditionKind::Single(kind) => kind.validate(current_address, knots),
        }
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        match self {
            ConditionKind::True | ConditionKind::False => true,
            ConditionKind::Nested(condition) => condition.all_addresses_are_valid(),
            ConditionKind::Single(kind) => kind.all_addresses_are_valid(),
        }
    }
}

impl ValidateAddresses for StoryCondition {
    fn validate(
        &mut self,
        current_address: &Address,
        knots: &Knots,
    ) -> Result<(), InvalidAddressError> {
        match self {
            StoryCondition::NumVisits {
                ref mut address, ..
            } => address.validate(current_address, knots),
        }
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        match self {
            StoryCondition::NumVisits { address, .. } => address.all_addresses_are_valid(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt;

    use ConditionKind::{False, True};

    impl From<StoryCondition> for Condition {
        fn from(kind: StoryCondition) -> Self {
            ConditionBuilder::from_item(&kind.into()).build()
        }
    }

    impl Condition {
        pub fn kind(&self) -> &StoryCondition {
            self.root.kind()
        }

        pub fn with_and(mut self, item: ConditionKind) -> Self {
            self.items.push(AndOr::And(item));
            self
        }

        pub fn with_or(mut self, item: ConditionKind) -> Self {
            self.items.push(AndOr::Or(item));
            self
        }
    }

    impl ConditionKind {
        pub fn kind(&self) -> &StoryCondition {
            match self {
                ConditionKind::Single(kind) => kind,
                other => panic!("tried to extract `StoryCondition`, but item was not `ConditionKind::Single` (was: {:?})", other),
            }
        }
    }

    impl AndOr {
        pub fn kind(&self) -> &StoryCondition {
            match self {
                AndOr::And(item) | AndOr::Or(item) => item.kind(),
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
        let f = |kind: &StoryCondition| match kind {
            _ => Err(MockError),
        };

        assert!(ConditionBuilder::from_item(&True.into())
            .build()
            .evaluate(&f)
            .unwrap());

        assert!(!ConditionBuilder::from_item(&False.into())
            .build()
            .evaluate(&f)
            .unwrap());

        assert!(ConditionBuilder::from_item(&True.into())
            .build()
            .with_and(True.into())
            .evaluate(&f)
            .unwrap());

        assert!(!ConditionBuilder::from_item(&True.into())
            .build()
            .with_and(False.into())
            .evaluate(&f)
            .unwrap());

        assert!(ConditionBuilder::from_item(&False.into())
            .build()
            .with_and(False.into())
            .with_or(True)
            .evaluate(&f)
            .unwrap());

        assert!(!ConditionBuilder::from_item(&False.into())
            .build()
            .with_and(False)
            .with_or(True)
            .with_and(False)
            .evaluate(&f)
            .unwrap());
    }
}
