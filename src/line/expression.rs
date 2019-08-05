use crate::{error::InklingError, follow::FollowData, line::Variable};

#[derive(Clone, Debug)]
pub struct Expression {
    pub head: Operand,
    pub tail: Vec<(Operation, Operand)>,
}

#[derive(Clone, Debug)]
pub enum Operand {
    Nested(Box<Expression>),
    Variable(Variable),
}

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

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

fn get_value(operand: &Operand, data: &FollowData) -> Result<Variable, InklingError> {
    match operand {
        Operand::Variable(variable) => variable.as_value(data),
        Operand::Nested(expression) => evaluate_expression(expression, data),
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
}
