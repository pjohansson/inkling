//! Expressions of numerical work or string concatenation of variables.

use crate::{
    error::{
        parse::validate::{ExpressionKind, InvalidVariableExpression, ValidationError},
        utils::MetaData,
        InklingError,
    },
    follow::FollowData,
    knot::Address,
    line::Variable,
    story::validate::{ValidateContent, ValidationData},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Single mathematical expression.
///
/// Consists of a head operand after which pairs of operators and operands appear.
/// In an expression `a + b + c`, `a` will be the head operand, with `+ b` and `+ c`
/// forming the tail.
pub struct Expression {
    /// Head term of expression.
    pub head: Operand,
    /// Tail terms of expression along with the operators operating on them.
    pub tail: Vec<(Operator, Operand)>,
}

impl Expression {
    pub fn add(&mut self, variable: Variable) {
        self.tail.push((Operator::Add, Operand::Variable(variable)));
    }

    pub fn sub(&mut self, variable: Variable) {
        self.tail
            .push((Operator::Subtract, Operand::Variable(variable)));
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Operand of an operation.
pub enum Operand {
    /// Nested inner expression from a parenthesis.
    Nested(Box<Expression>),
    /// Variable with a value.
    Variable(Variable),
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Mathematical operator applied to a term.
///
/// In strings these operators are assigned to values on the right of them.
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

/// Evaluate an expression from start to finish, producing a single `Variable` value.
pub fn evaluate_expression(
    expression: &Expression,
    data: &FollowData,
) -> Result<Variable, InklingError> {
    expression
        .tail
        .iter()
        .map(|(operation, operand)| (operation, get_value(operand, data)))
        .fold(
            get_value(&expression.head, data),
            |acc, (operation, operand)| {
                let lhs_variable = acc?;
                let rhs_variable = operand?;

                match operation {
                    Operator::Add => lhs_variable.add(&rhs_variable),
                    Operator::Subtract => lhs_variable.subtract(&rhs_variable),
                    Operator::Multiply => lhs_variable.multiply(&rhs_variable),
                    Operator::Divide => lhs_variable.divide(&rhs_variable),
                    Operator::Remainder => lhs_variable.remainder(&rhs_variable),
                }
                .map_err(|err| err.into())
            },
        )
}

/// Nest inner operations based on order of precedence in operations.
///
/// Subitems which multiply, divide or take the remainder with their next item are grouped
/// in `Operand::Nested` containers as single units, to be evaluated as a whole before
/// addition and subtraction.
///
/// This operation corresponds to inserting parenthesis around these groups.
///
/// # Notes
/// *   This function does *not* recurse into nested expressions to apply order of operations
///     for them. This has to be done separately, as those items are created.
pub fn apply_order_of_operations(expression: &Expression) -> Expression {
    split_expression_into_groups_of_same_precedence(expression)
        .into_iter()
        .map(|group| get_maybe_nested_operand_from_group(group))
        .collect::<Vec<_>>()
        .split_first()
        .map(|((_, head), tail)| Expression {
            head: head.clone(),
            tail: tail.to_vec(),
        })
        .unwrap()
}

/// Evaluate a variable or inner expression to produce a single variable.
fn get_value(operand: &Operand, data: &FollowData) -> Result<Variable, InklingError> {
    match operand {
        Operand::Nested(expression) => evaluate_expression(expression, data),
        Operand::Variable(variable) => variable.as_value(data),
    }
}

/// Split the expression items into groups, divided by addition and subtraction.
///
/// This groups multiplied, divided with and remainder or items, while added and subtracted
/// items remain alone.
fn split_expression_into_groups_of_same_precedence(
    expression: &Expression,
) -> Vec<Vec<(Operator, Operand)>> {
    let mut items = vec![(Operator::Add, expression.head.clone())];
    items.extend_from_slice(&expression.tail);

    let mut groups = Vec::new();
    let mut group = Vec::new();

    for (operation, operand) in items {
        match operation {
            Operator::Add | Operator::Subtract if !group.is_empty() => {
                groups.push(group);
                group = Vec::new();
            }
            _ => (),
        }

        group.push((operation, operand));
    }

    if !group.is_empty() {
        groups.push(group);
    }

    groups
}

/// Create `Variable` or `Nested` variants from a group.
///
/// If the group contains a single item it will be returned as an `Operand::Variable` object.
/// If not, a `Operand::Nested` object is constructed from all items.
fn get_maybe_nested_operand_from_group(group: Vec<(Operator, Operand)>) -> (Operator, Operand) {
    if group.len() == 1 {
        group[0].clone()
    } else {
        group
            .split_first()
            .map(|((operation, head), tail)| {
                let expression = Expression {
                    head: head.clone(),
                    tail: tail.to_vec(),
                };

                (*operation, Operand::Nested(Box::new(expression)))
            })
            .unwrap()
    }
}

impl ValidateContent for Expression {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        current_location: &Address,
        meta_data: &MetaData,
        data: &ValidationData,
    ) {
        let num_errors = error.num_errors();

        self.head.validate(error, current_location, meta_data, data);

        self.tail
            .iter_mut()
            .for_each(|(_, operand)| operand.validate(error, current_location, meta_data, data));

        if num_errors == error.num_errors() {
            if let Err(err) = evaluate_expression(self, &data.follow_data) {
                error.variable_errors.push(InvalidVariableExpression {
                    expression_kind: ExpressionKind::Expression,
                    kind: err.into(),
                    meta_data: meta_data.clone(),
                });
            }
        }
    }
}

impl ValidateContent for Operand {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        current_location: &Address,
        meta_data: &MetaData,
        data: &ValidationData,
    ) {
        match self {
            Operand::Nested(ref mut expression) => {
                expression.validate(error, current_location, meta_data, data)
            }
            Operand::Variable(ref mut variable) => {
                variable.validate(error, current_location, meta_data, data)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{follow::FollowDataBuilder, knot::Address, story::types::VariableInfo};

    use std::collections::HashMap;

    impl From<Variable> for Expression {
        fn from(variable: Variable) -> Self {
            Expression {
                head: Operand::Variable(variable),
                tail: Vec::new(),
            }
        }
    }

    fn get_simple_expression(head: Variable, tail: &[(Operator, Variable)]) -> Expression {
        let tail = tail
            .iter()
            .cloned()
            .map(|(operation, operand)| (operation, Operand::Variable(operand)))
            .collect();

        Expression {
            head: Operand::Variable(head),
            tail,
        }
    }

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

        FollowDataBuilder::new()
            .with_knots(knot_visit_counts)
            .with_variables(variables)
            .build()
    }

    #[test]
    fn expression_can_add_and_subtract_values_with_methods() {
        let mut expression = get_simple_expression(Variable::Int(5), &[]);

        expression.add(Variable::Int(2));
        expression.sub(Variable::Int(1));

        assert_eq!(
            expression.tail[0],
            (Operator::Add, Operand::Variable(Variable::Int(2)))
        );
        assert_eq!(
            expression.tail[1],
            (Operator::Subtract, Operand::Variable(Variable::Int(1)))
        );
    }

    #[test]
    fn expression_with_just_head_evaluates_to_head() {
        let data = mock_follow_data(&[], &[]);
        let expression = get_simple_expression(Variable::Int(5), &[]);

        assert_eq!(
            evaluate_expression(&expression, &data).unwrap(),
            Variable::Int(5)
        );
    }

    #[test]
    fn adding_two_variables_creates_summed_variable() {
        let data = mock_follow_data(&[], &[]);

        let expression =
            get_simple_expression(Variable::Int(1), &[(Operator::Add, Variable::Int(2))]);

        assert_eq!(
            evaluate_expression(&expression, &data).unwrap(),
            Variable::Int(3)
        );
    }

    #[test]
    fn all_operations_work_in_order() {
        let data = mock_follow_data(&[], &[]);

        // 1 + 2 - (-2) * (-3) / 5 = -3
        let expression = get_simple_expression(
            Variable::Float(1.0),
            &[
                (Operator::Add, Variable::Float(2.0)),
                (Operator::Subtract, Variable::Float(-2.0)),
                (Operator::Multiply, Variable::Float(-3.0)),
                (Operator::Divide, Variable::Float(5.0)),
            ],
        );

        assert_eq!(
            evaluate_expression(&expression, &data).unwrap(),
            Variable::Float(-3.0)
        );
    }

    #[test]
    fn get_value_evaluates_variables_by_following_addresses_if_necessary() {
        let data = mock_follow_data(&[], &[("counter", 1.into())]);

        let variable = Variable::Address(Address::variable_unchecked("counter"));

        assert_eq!(
            get_value(&Operand::Variable(variable), &data).unwrap(),
            Variable::Int(1)
        );
    }

    #[test]
    fn nested_expression_evaluates_into_variable() {
        let data = mock_follow_data(&[], &[]);

        let nested_expression = get_simple_expression(
            Variable::Int(1),
            &[(Operator::Multiply, Variable::Float(3.9))],
        );

        let nested = Operand::Nested(Box::new(nested_expression.clone()));

        assert_eq!(
            evaluate_expression(&nested_expression, &data).unwrap(),
            get_value(&nested, &data).unwrap()
        );
    }

    #[test]
    fn order_of_operations_on_expression_with_just_head_is_head() {
        let expression = get_simple_expression(Variable::Int(5), &[]);
        assert_eq!(apply_order_of_operations(&expression), expression);
    }

    #[test]
    fn order_of_operations_with_just_add_and_subtract_changes_nothing() {
        // 1 + 1 - 2 - 3 + 4 = 1 + 1 - 2 - 3 + 4
        let expression = get_simple_expression(
            1.into(),
            &[
                (Operator::Add, 1.into()),
                (Operator::Subtract, 2.into()),
                (Operator::Subtract, 3.into()),
                (Operator::Add, 4.into()),
            ],
        );

        let ooo_expression = apply_order_of_operations(&expression);

        assert_eq!(ooo_expression, expression);
    }

    #[test]
    fn order_of_operations_gathers_multiplied_items_into_nested_groups() {
        // 1 + 1 * 2 * 3 = 1 + (1 * 2 * 3)
        let expression = get_simple_expression(
            1.into(),
            &[
                (Operator::Add, 1.into()),
                (Operator::Multiply, 2.into()),
                (Operator::Multiply, 3.into()),
            ],
        );

        let nested_expression = get_simple_expression(
            1.into(),
            &[
                (Operator::Multiply, 2.into()),
                (Operator::Multiply, 3.into()),
            ],
        );

        let nested = Operand::Nested(Box::new(nested_expression));

        let ooo_expression = apply_order_of_operations(&expression);

        assert_eq!(ooo_expression.tail[0], (Operator::Add, nested));
    }

    #[test]
    fn multiple_nested_groups_are_separated_by_addition_or_subtraction() {
        // 1 + 1 * 2 + 3 * 4 = 1 + (1 * 2) + (3 * 4)
        let expression = get_simple_expression(
            1.into(),
            &[
                (Operator::Add, 1.into()),
                (Operator::Multiply, 2.into()),
                (Operator::Add, 3.into()),
                (Operator::Multiply, 4.into()),
            ],
        );

        let nested_expression_one =
            get_simple_expression(1.into(), &[(Operator::Multiply, 2.into())]);

        let nested_expression_two =
            get_simple_expression(3.into(), &[(Operator::Multiply, 4.into())]);

        let nested_one = Operand::Nested(Box::new(nested_expression_one));
        let nested_two = Operand::Nested(Box::new(nested_expression_two));

        let ooo_expression = apply_order_of_operations(&expression);

        assert_eq!(ooo_expression.tail[0], (Operator::Add, nested_one));
        assert_eq!(ooo_expression.tail[1], (Operator::Add, nested_two));
    }

    #[test]
    fn if_all_items_are_multiply_they_gather_into_one_item_in_head() {
        // 1 * 2 * 3 = (1 * 2 * 3)
        let expression = get_simple_expression(
            1.into(),
            &[
                (Operator::Multiply, 1.into()),
                (Operator::Multiply, 2.into()),
            ],
        );

        let nested = Operand::Nested(Box::new(expression.clone()));

        let ooo_expression = apply_order_of_operations(&expression);

        assert_eq!(ooo_expression.head, nested);
    }
}
