use crate::{error::InklingError, follow::FollowData, line::Variable};

#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    pub head: Operand,
    pub tail: Vec<(Operation, Operand)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operand {
    Nested(Box<Expression>),
    Variable(Variable),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

/// Evaluate an expression, producing a single `Variable` value.
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
                    Operation::Add => lhs_variable.add(&rhs_variable),
                    Operation::Subtract => lhs_variable.subtract(&rhs_variable),
                    Operation::Multiply => lhs_variable.multiply(&rhs_variable),
                    Operation::Divide => lhs_variable.divide(&rhs_variable),
                    Operation::Remainder => lhs_variable.remainder(&rhs_variable),
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
) -> Vec<Vec<(Operation, Operand)>> {
    let mut items = vec![(Operation::Add, expression.head.clone())];
    items.extend_from_slice(&expression.tail);

    let mut groups = Vec::new();
    let mut group = Vec::new();

    for (operation, operand) in items {
        match operation {
            Operation::Add | Operation::Subtract if !group.is_empty() => {
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
fn get_maybe_nested_operand_from_group(group: Vec<(Operation, Operand)>) -> (Operation, Operand) {
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::knot::Address;

    use std::collections::HashMap;

    fn get_simple_expression(head: Variable, tail: &[(Operation, Variable)]) -> Expression {
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
            .map(|(name, var)| (name.to_string(), var))
            .collect();

        FollowData {
            knot_visit_counts,
            variables,
        }
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
            get_simple_expression(Variable::Int(1), &[(Operation::Add, Variable::Int(2))]);

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
                (Operation::Add, Variable::Float(2.0)),
                (Operation::Subtract, Variable::Float(-2.0)),
                (Operation::Multiply, Variable::Float(-3.0)),
                (Operation::Divide, Variable::Float(5.0)),
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
            &[(Operation::Multiply, Variable::Float(3.9))],
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
                (Operation::Add, 1.into()),
                (Operation::Subtract, 2.into()),
                (Operation::Subtract, 3.into()),
                (Operation::Add, 4.into()),
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
                (Operation::Add, 1.into()),
                (Operation::Multiply, 2.into()),
                (Operation::Multiply, 3.into()),
            ],
        );

        let nested_expression = get_simple_expression(
            1.into(),
            &[
                (Operation::Multiply, 2.into()),
                (Operation::Multiply, 3.into()),
            ],
        );

        let nested = Operand::Nested(Box::new(nested_expression));

        let ooo_expression = apply_order_of_operations(&expression);

        assert_eq!(ooo_expression.tail[0], (Operation::Add, nested));
    }

    #[test]
    fn multiple_nested_groups_are_separated_by_addition_or_subtraction() {
        // 1 + 1 * 2 + 3 * 4 = 1 + (1 * 2) + (3 * 4)
        let expression = get_simple_expression(
            1.into(),
            &[
                (Operation::Add, 1.into()),
                (Operation::Multiply, 2.into()),
                (Operation::Add, 3.into()),
                (Operation::Multiply, 4.into()),
            ],
        );

        let nested_expression_one =
            get_simple_expression(1.into(), &[(Operation::Multiply, 2.into())]);

        let nested_expression_two =
            get_simple_expression(3.into(), &[(Operation::Multiply, 4.into())]);

        let nested_one = Operand::Nested(Box::new(nested_expression_one));
        let nested_two = Operand::Nested(Box::new(nested_expression_two));

        let ooo_expression = apply_order_of_operations(&expression);

        assert_eq!(ooo_expression.tail[0], (Operation::Add, nested_one));
        assert_eq!(ooo_expression.tail[1], (Operation::Add, nested_two));
    }

    #[test]
    fn if_all_items_are_multiply_they_gather_into_one_item_in_head() {
        // 1 * 2 * 3 = (1 * 2 * 3)
        let expression = get_simple_expression(
            1.into(),
            &[
                (Operation::Multiply, 1.into()),
                (Operation::Multiply, 2.into()),
            ],
        );

        let nested = Operand::Nested(Box::new(expression.clone()));

        let ooo_expression = apply_order_of_operations(&expression);

        assert_eq!(ooo_expression.head, nested);
    }
}
