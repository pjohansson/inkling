//! Checking of `Condition`s which determine whether content will be displayed.

use crate::{
    error::{
        variable::{VariableError, VariableErrorKind},
        InklingError,
    },
    follow::FollowData,
    line::{expression::evaluate_expression, Condition, StoryCondition, Variable},
};

use std::cmp::Ordering;

/// Check whether a single condition is fulfilled.
pub fn check_condition(condition: &Condition, data: &FollowData) -> Result<bool, InklingError> {
    let evaluator = |kind: &StoryCondition| match kind {
        StoryCondition::Comparison {
            lhs_variable,
            rhs_variable,
            ordering,
        } => {
            let lhs = evaluate_expression(lhs_variable, data)?;
            let rhs = evaluate_expression(rhs_variable, data)?;

            match ordering {
                Ordering::Equal => lhs.equal_to(&rhs),
                Ordering::Greater => lhs.greater_than(&rhs),
                Ordering::Less => lhs.less_than(&rhs),
            }
        }
        .map_err(|err| err.into()),
        StoryCondition::IsTrueLike { variable } => match variable.as_value(data)? {
            Variable::Bool(value) => Ok(value),
            Variable::Float(value) => Ok(value != 0.0),
            Variable::Int(value) => Ok(value != 0),
            Variable::String(s) => Ok(s.len() > 0),
            Variable::Divert(..) => Err(VariableError::from_kind(
                variable.clone(),
                VariableErrorKind::InvalidComparison {
                    other: Variable::Bool(true),
                    comparison: Ordering::Equal,
                },
            )
            .into()),
            Variable::Address(..) => unreachable!("`as_value` will not return an `Address`"),
        },
    };

    condition.evaluate(&evaluator)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        knot::Address,
        line::{
            expression::{Expression, Operand},
            ConditionBuilder,
        },
        story::types::VariableInfo,
    };

    use std::collections::HashMap;

    fn mock_follow_data(knots: &[(&str, &str, u32)], variables: &[(&str, Variable)]) -> FollowData {
        let mut knot_visit_counts = HashMap::new();

        for (knot, stitch, num_visited) in knots {
            let mut stitch_count = HashMap::new();
            stitch_count.insert(stitch.to_string(), *num_visited);

            knot_visit_counts.insert(knot.to_string(), stitch_count);
        }

        let variables = variables
            .into_iter()
            .cloned()
            .enumerate()
            .map(|(i, (name, var))| (name, VariableInfo::new(var, i)))
            .map(|(name, var)| (name.to_string(), var))
            .collect();

        FollowData {
            knot_visit_counts,
            variables,
        }
    }

    fn get_true_like_condition(variable: Variable, negate: bool) -> Condition {
        let kind = StoryCondition::IsTrueLike { variable };

        ConditionBuilder::from_kind(&kind.into(), negate).build()
    }

    fn get_variable_comparison_condition(
        lhs_variable: Variable,
        rhs_variable: Variable,
        ordering: Ordering,
        negate: bool,
    ) -> Condition {
        let kind = StoryCondition::Comparison {
            lhs_variable: Expression {
                head: Operand::Variable(lhs_variable),
                tail: Vec::new(),
            },
            rhs_variable: Expression {
                head: Operand::Variable(rhs_variable),
                tail: Vec::new(),
            },
            ordering,
        };

        ConditionBuilder::from_kind(&kind.into(), negate).build()
    }

    #[test]
    fn conditions_can_compare_variable_values() {
        let data = mock_follow_data(&[], &[]);

        let integer_condition = get_variable_comparison_condition(
            Variable::from(5),
            Variable::from(6),
            Ordering::Less,
            false,
        );

        let string_condition = get_variable_comparison_condition(
            Variable::from("hi"),
            Variable::from("hiya"),
            Ordering::Equal,
            false,
        );

        assert!(check_condition(&integer_condition, &data).unwrap());
        assert!(!check_condition(&string_condition, &data).unwrap());
    }

    #[test]
    fn is_true_like_conditions_return_true_if_variable_is_boolean_and_true() {
        let data = mock_follow_data(&[], &[]);

        let true_condition = get_true_like_condition(Variable::from(true), false);
        let false_condition = get_true_like_condition(Variable::from(false), false);

        assert!(check_condition(&true_condition, &data).unwrap());
        assert!(!check_condition(&false_condition, &data).unwrap());
    }

    #[test]
    fn is_true_like_conditions_return_true_if_variable_is_numeric_and_non_zero() {
        let data = mock_follow_data(&[], &[]);

        let int_equal = get_true_like_condition(Variable::from(0), false);
        let int_greater = get_true_like_condition(Variable::from(1), false);
        let int_less = get_true_like_condition(Variable::from(-1), false);

        assert!(check_condition(&int_greater, &data).unwrap());
        assert!(check_condition(&int_less, &data).unwrap());
        assert!(!check_condition(&int_equal, &data).unwrap());

        let float_equal = get_true_like_condition(Variable::from(0.0), false);
        let float_greater = get_true_like_condition(Variable::from(0.1), false);
        let float_less = get_true_like_condition(Variable::from(-0.1), false);

        assert!(check_condition(&float_greater, &data).unwrap());
        assert!(check_condition(&float_less, &data).unwrap());
        assert!(!check_condition(&float_equal, &data).unwrap());
    }

    #[test]
    fn is_true_like_conditions_return_true_if_variable_is_string_with_non_zero_length() {
        let data = mock_follow_data(&[], &[]);

        let string_word = get_true_like_condition(Variable::from("non-empty"), false);
        let string_char = get_true_like_condition(Variable::from("c"), false);
        let string_empty = get_true_like_condition(Variable::from(""), false);

        assert!(check_condition(&string_word, &data).unwrap());
        assert!(check_condition(&string_char, &data).unwrap());
        assert!(!check_condition(&string_empty, &data).unwrap());
    }

    #[test]
    fn is_true_like_condition_yields_error_if_variable_is_divert() {
        let data = mock_follow_data(&[("tripoli", "cinema", 1)], &[]);

        let variable = Variable::Divert(Address::from_parts_unchecked("tripoli", Some("cinema")));
        let divert = get_true_like_condition(variable, false);

        assert!(check_condition(&divert, &data).is_err());
    }
}
