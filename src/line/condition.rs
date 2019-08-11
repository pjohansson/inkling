//! Conditions for displaying choices, lines or other content.
//!
//! The base of this module is the `Condition` struct which is in essence the root
//! of a set of conditions which must be fulfilled for some content to be displayed.
//! From this root we can evaluate the entire condition tree linked to it.
//!
//! Since `Condition` is the large container for a condition, there are several
//! smaller pieces working as the glue. `ConditionItem` is a container for each
//! individual part of a condition.
//!
//! For example, if a condition is `(i > 2) and (i < 5)` then the entire string
//! represents the `Condition` while the individual `i > 2` and `i < 5` parts are
//! `ConditionItem`. Each individual `ConditionItem` can be negated: `not i > 2`,
//! and so on.
//!
//! `Ink` supports two types of links between conditions: `and` and `or` (no exclusive
//! or). These are linked to `ConditionItem`s through the `AndOr` struct. So when
//! the full `Condition` is evaluating it will check this enum along with the item
//! to assert whether the condition passes.
//!
//! Finally comes the representation of single statements. These are contained in
//! the `ConditionKind` enum which has items for `true` and `false` if a super
//! simple item is created, `StoryCondition` if the condition has to access the
//! running story state to be evaluated (this will almost always be the case)
//! and `Nested` for nested conditions.
//!
//! A note about `StoryCondition`: this represents a condition created by the user
//! in the script. This module is not responsible for evaluating it based on
//! the story state. The module is responsible for ensuring that conditions and logic
//! works correctly through nesting and whatnot. See `Condition` and its methods for
//! more information.

use crate::{
    error::{parse::address::InvalidAddressError, utils::MetaData},
    knot::{Address, ValidateAddressData, ValidateAddresses},
    line::{Expression, Variable},
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
    pub root: ConditionItem,
    /// Ordered set of `and`/`or` conditions to compare the first condition to.
    pub items: Vec<AndOr>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Base item in a condition.
///
/// Will evaluate to a single `true` or `false` but may have to evaluate a group
/// of conditions. This is not done by this module or struct! This struct only
/// implements the framework through which choices can be created, parsed and
/// ensured that all items in a condition are true if told.
///
/// The evaluation of each individual condition is performed by the `evaluate`
/// method. This takes a closure for the caller and applies it to the item,
/// producing the result which is linked to the rest of the conditions to
/// determine the final true or false value.
pub struct ConditionItem {
    /// Negate the condition upon evaluation.
    pub negate: bool,
    /// Kind of condition.
    pub kind: ConditionKind,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Condition variants.
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
    /// Compares two variables from an `x > y` (or similar) comparative statement.
    ///
    /// Variable `Address` variants will evaluate to their value (see the `as_value`
    /// method for [`Variable`][crate::line::Variable]), then compare using that.
    ///
    /// Equal-to comparisons (`==`) can be made for all variable types. Less-than (`<`)
    /// and greater-than (`>`) comparisons are only allowed for `Int` and `Float` variants.
    /// An error is raised if another variant is used like that.
    Comparison {
        /// Left hand side variable.
        lhs_variable: Expression,
        /// Right hand side variable.
        rhs_variable: Expression,
        /// Order comparison between the two.
        ///
        /// Applies from the left hand side variable to the right hand side. Meaning that
        /// for eg. `lhs > rhs` the ordering will be `Ordering::Greater`.
        #[cfg_attr(feature = "serde_support", serde(with = "OrderingDerive"))]
        ordering: Ordering,
    },
    /// Assert that the variable value is "true".
    ///
    /// This is evaluated differently for different variable types.
    ///
    /// *   Boolean variables evaluate directly.
    /// *   Number variables (integers and floats) are `true` if they are non-zero.
    /// *   String variables are `true` if they have non-zero length.
    ///
    /// Variable `Address` variants will evaluate their value (see the `as_value` method
    /// for [`Variable`][crate::line::Variable]), then as above.
    ///
    /// Variable `Divert` variants will never evaluate to `true` or `false`, but raise
    /// and error. They are not supposed to be used like this.
    IsTrueLike { variable: Variable },
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Container for `and`/`or` variants of conditions to evaluate in a list.
pub enum AndOr {
    And(ConditionItem),
    Or(ConditionItem),
}

impl Condition {
    /// Evaluate the condition with the given evaluator closure.
    ///
    /// This closure will be called on every item in the `Condition` as all parts
    /// are walked through.
    pub fn evaluate<F, E>(&self, evaluator: &F) -> Result<bool, E>
    where
        F: Fn(&StoryCondition) -> Result<bool, E>,
        E: Error,
    {
        self.items
            .iter()
            .fold(inner_eval(&self.root, evaluator), |acc, next_condition| {
                acc.and_then(|current| match next_condition {
                    AndOr::And(item) => inner_eval(item, evaluator).map(|next| current && next),
                    AndOr::Or(item) => inner_eval(item, evaluator).map(|next| current || next),
                })
            })
    }
}

/// Match against and evaluate the items.
fn inner_eval<F, E>(item: &ConditionItem, evaluator: &F) -> Result<bool, E>
where
    F: Fn(&StoryCondition) -> Result<bool, E>,
    E: Error,
{
    let mut result = match &item.kind {
        ConditionKind::True => Ok(true),
        ConditionKind::False => Ok(false),
        ConditionKind::Nested(condition) => condition.evaluate(evaluator),
        ConditionKind::Single(ref kind) => evaluator(kind),
    }?;

    if item.negate {
        result = !result;
    }

    Ok(result)
}

/// Constructor struct for `Condition`.
pub struct ConditionBuilder {
    root: ConditionItem,
    items: Vec<AndOr>,
}

impl ConditionBuilder {
    /// Create the constructor with a condition kind.
    pub fn from_kind(kind: &ConditionKind, negate: bool) -> Self {
        let root = ConditionItem {
            kind: kind.clone(),
            negate,
        };

        ConditionBuilder {
            root,
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
    pub fn and(&mut self, kind: &ConditionKind, negate: bool) {
        self.items.push(AndOr::And(ConditionItem {
            kind: kind.clone(),
            negate,
        }));
    }

    /// Add an `or` item to the condition list.
    pub fn or(&mut self, kind: &ConditionKind, negate: bool) {
        self.items.push(AndOr::Or(ConditionItem {
            kind: kind.clone(),
            negate,
        }));
    }
}

impl ValidateAddresses for Condition {
    fn validate(
        &mut self,
        errors: &mut Vec<InvalidAddressError>,
        meta_data: &MetaData,
        current_address: &Address,
        data: &ValidateAddressData,
    ) {
        self.root
            .kind
            .validate(errors, meta_data, current_address, data);

        self.items.iter_mut().for_each(|item| match item {
            AndOr::And(item) | AndOr::Or(item) => {
                item.kind.validate(errors, meta_data, current_address, data)
            }
        });
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        self.root.kind.all_addresses_are_valid()
            && self.items.iter().all(|item| match item {
                AndOr::And(item) | AndOr::Or(item) => item.kind.all_addresses_are_valid(),
            })
    }
}

impl ValidateAddresses for ConditionKind {
    fn validate(
        &mut self,
        errors: &mut Vec<InvalidAddressError>,
        meta_data: &MetaData,
        current_address: &Address,
        data: &ValidateAddressData,
    ) {
        match self {
            ConditionKind::True | ConditionKind::False => (),
            ConditionKind::Nested(condition) => {
                condition.validate(errors, meta_data, current_address, data)
            }
            ConditionKind::Single(kind) => kind.validate(errors, meta_data, current_address, data),
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
        errors: &mut Vec<InvalidAddressError>,
        meta_data: &MetaData,
        current_address: &Address,
        data: &ValidateAddressData,
    ) {
        match self {
            StoryCondition::Comparison {
                ref mut lhs_variable,
                ref mut rhs_variable,
                ..
            } => {
                lhs_variable.validate(errors, meta_data, current_address, data);
                rhs_variable.validate(errors, meta_data, current_address, data);
            }
            StoryCondition::IsTrueLike { variable } => {
                variable.validate(errors, meta_data, current_address, data)
            }
        }
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        match self {
            StoryCondition::Comparison {
                lhs_variable,
                rhs_variable,
                ..
            } => lhs_variable.all_addresses_are_valid() && rhs_variable.all_addresses_are_valid(),
            StoryCondition::IsTrueLike { variable } => variable.all_addresses_are_valid(),
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
            ConditionBuilder::from_kind(&kind.into(), false).build()
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

    impl Condition {
        pub fn story_condition(&self) -> &StoryCondition {
            &self.root.kind.story_condition()
        }

        pub fn with_and(mut self, kind: ConditionKind) -> Self {
            let item = ConditionItem {
                kind,
                negate: false,
            };

            self.items.push(AndOr::And(item));
            self
        }

        pub fn with_or(mut self, kind: ConditionKind) -> Self {
            let item = ConditionItem {
                kind,
                negate: false,
            };

            self.items.push(AndOr::Or(item));
            self
        }
    }

    impl ConditionKind {
        pub fn nested(&self) -> &Condition {
            match self {
                ConditionKind::Nested(condition) => condition,
                other => panic!(
                    "tried to extract nested `Condition`, but item was not `ConditionKind::Nested` \
                     (was: {:?})",
                     other
                ),
            }
        }

        pub fn story_condition(&self) -> &StoryCondition {
            match self {
                ConditionKind::Single(story_condition) => story_condition,
                other => panic!(
                    "tried to extract `StoryCondition`, but item was not `ConditionKind::Single` \
                     (was: {:?})",
                    other
                ),
            }
        }
    }

    impl AndOr {
        pub fn nested(&self) -> &Condition {
            match self {
                AndOr::And(item) | AndOr::Or(item) => item.kind.nested(),
            }
        }

        pub fn story_condition(&self) -> &StoryCondition {
            match self {
                AndOr::And(item) | AndOr::Or(item) => item.kind.story_condition(),
            }
        }

        pub fn is_and(&self) -> bool {
            match self {
                AndOr::And(..) => true,
                _ => false,
            }
        }

        pub fn is_or(&self) -> bool {
            match self {
                AndOr::Or(..) => true,
                _ => false,
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

        assert!(ConditionBuilder::from_kind(&True.into(), false)
            .build()
            .evaluate(&f)
            .unwrap());

        assert!(!ConditionBuilder::from_kind(&False.into(), false)
            .build()
            .evaluate(&f)
            .unwrap());

        assert!(ConditionBuilder::from_kind(&True.into(), false)
            .build()
            .with_and(True.into())
            .evaluate(&f)
            .unwrap());

        assert!(!ConditionBuilder::from_kind(&True.into(), false)
            .build()
            .with_and(False.into())
            .evaluate(&f)
            .unwrap());

        assert!(ConditionBuilder::from_kind(&False.into(), false)
            .build()
            .with_and(False.into())
            .with_or(True)
            .evaluate(&f)
            .unwrap());

        assert!(!ConditionBuilder::from_kind(&False.into(), false)
            .build()
            .with_and(False)
            .with_or(True)
            .with_and(False)
            .evaluate(&f)
            .unwrap());
    }

    #[test]
    fn conditions_can_be_negated() {
        let f = |kind: &StoryCondition| match kind {
            _ => Err(MockError),
        };

        assert!(ConditionBuilder::from_kind(&False.into(), true)
            .build()
            .evaluate(&f)
            .unwrap());
    }
}
