use crate::{
    error::{LineErrorKind, LineParsingError},
    line::{
        expression::{apply_order_of_operations, Operand, Operation},
        parse::{parse_variable, split_line_at_separator_parenthesis},
        Expression,
    },
};

const OPERATORS: &[char] = &['+', '-', '*', '/', '%'];

pub fn parse_expression(content: &str) -> Result<Expression, LineParsingError> {
    split_line_into_operations(content)
        .and_then(|operations| parse_expression_from_operations(operations))
        .map(|expression| apply_order_of_operations(&expression))
}

fn parse_expression_from_operations(
    mut operation_strings: Vec<String>,
) -> Result<Expression, LineParsingError> {
    operation_strings
        .split_first_mut()
        .map(|(head_string, tail_items)| {
            let head = parse_operand(head_string.trim())?;

            let tail = tail_items
                .into_iter()
                .map(|mut content| {
                    let operator = split_off_operator(&mut content);
                    let operand = parse_operand(content.trim())?;

                    Ok((operator, operand))
                })
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Expression { head, tail })
        })
        .unwrap()
}

/// Parse the `Operand` from an expression.
///
/// Assumes that the given string is trimmed of whitespace from both ends.
fn parse_operand(content: &str) -> Result<Operand, LineParsingError> {
    if content.starts_with('(') && content.ends_with(')') && content.len() > 1 {
        let inner = content.get(1..content.bytes().len() - 1).unwrap();
        let expression = parse_expression(inner)?;

        Ok(Operand::Nested(Box::new(expression)))
    } else {
        parse_variable(content)
            .map_err(|kind| LineParsingError::from_kind(content, kind))
            .map(|variable| Operand::Variable(variable))
    }
}

fn split_line_into_operations(content: &str) -> Result<Vec<String>, LineParsingError> {
    let mut buffer = content.trim().to_string();
    let mut operations = Vec::new();

    while !buffer.trim().is_empty() {
        let operation_string = read_next_operation_string(&mut buffer)
            .map_err(|kind| LineParsingError::from_kind(content, kind))?;

        operations.push(operation_string);
    }

    Ok(operations)
}

/// Split off the initial operator and return its type.
///
/// Assumes to be called on lines for which operators were definitely found. This should
/// always be the case, since we split the lines where we find the operators.
fn split_off_operator(buffer: &mut String) -> Operation {
    buffer
        .drain(..1)
        .next()
        .map(|c| match c {
            '+' => Operation::Add,
            '-' => Operation::Subtract,
            '*' => Operation::Multiply,
            '/' => Operation::Divide,
            '%' => Operation::Remainder,
            _ => unreachable!(),
        })
        .unwrap()
}

/// Split the string corresponding to the next whole operation from the buffer.
///
/// Splits occur when mathematical operators '+', '-', '*', '/' and '%' are encountered
/// outside of parenthesis and strings (marked by '""' marks).
fn read_next_operation_string(buffer: &mut String) -> Result<String, LineErrorKind> {
    let (head, tail) = get_without_starting_match(&buffer);
    let head_size = head.len();

    let mut last_index = 0;

    let index = loop {
        let haystack = tail.get(last_index..).unwrap();

        let i = get_closest_split_index(haystack).map_err(|_| LineErrorKind::BadExpression)?;

        let index = i + last_index;

        if buffer
            .get(..index + 1)
            .map(|s| s.matches(|c| c == '"').count() % 2 == 0)
            .unwrap_or(true)
            || index >= tail.bytes().len()
        {
            break index;
        } else {
            last_index += i + 1;
        }
    };

    Ok(buffer.drain(..index + head_size).collect())
}

/// Trim last leading mathematical operator from line.
///
/// Assumes that the given string is trimmed from the start.
fn get_without_starting_match(content: &str) -> (&str, &str) {
    let index = if content
        .chars()
        .next()
        .map(|c| OPERATORS.contains(&c))
        .unwrap_or(false)
    {
        1
    } else {
        0
    };

    content.split_at(index)
}

/// Return the lowest index for any mathematical operator in a line.
fn get_closest_split_index(content: &str) -> Result<usize, LineParsingError> {
    get_split_index(content, "+")
        .and_then(|current_min| get_split_index(&content, "-").map(|next| current_min.min(next)))
        .and_then(|current_min| get_split_index(&content, "*").map(|next| current_min.min(next)))
        .and_then(|current_min| get_split_index(&content, "/").map(|next| current_min.min(next)))
        .and_then(|current_min| get_split_index(&content, "%").map(|next| current_min.min(next)))
}

/// Return the lowest index for the given separator keyword in the line.
fn get_split_index(content: &str, separator: &str) -> Result<usize, LineParsingError> {
    split_line_at_separator_parenthesis(content, separator, Some(1))
        .map(|parts| parts[0].as_bytes().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        follow::FollowData,
        line::{evaluate_expression, Variable},
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
            .map(|(name, var)| (name.to_string(), var))
            .collect();

        FollowData {
            knot_visit_counts,
            variables,
        }
    }

    #[test]
    fn single_number_parses_into_expression_with_only_head() {
        let expression = parse_expression("5").unwrap();

        assert_eq!(expression.head, Operand::Variable(Variable::Int(5)));
        assert!(expression.tail.is_empty());
    }

    #[test]
    fn number_then_operand_then_number_parses_into_addition_expression() {
        let expression = parse_expression("1 + 2").unwrap();

        assert_eq!(expression.head, Operand::Variable(Variable::Int(1)));

        assert_eq!(expression.tail.len(), 1);

        assert_eq!(
            expression.tail[0],
            (Operation::Add, Operand::Variable(Variable::Int(2)))
        );
    }

    #[test]
    fn many_operations_created_nested_structure_based_on_operator_precedence() {
        let data = mock_follow_data(&[], &[]);

        let expression = parse_expression("1 + 2 - 2 * 3 + 1 / 5 + 5").unwrap();
        let equiv_expression = parse_expression("1 + 2 - (2 * 3) + (1 / 5) + 5").unwrap();

        assert_eq!(
            evaluate_expression(&expression, &data).unwrap(),
            evaluate_expression(&equiv_expression, &data).unwrap()
        );
    }

    #[test]
    fn whitespace_does_not_matter() {
        let data = mock_follow_data(&[], &[]);

        let expression = parse_expression("1 + 2 - 2 * 3 + 1 / 5 + 5").unwrap();
        let equiv_expression = parse_expression("1+2-(2*3)+(1/5)+5").unwrap();

        assert_eq!(
            evaluate_expression(&expression, &data).unwrap(),
            evaluate_expression(&equiv_expression, &data).unwrap()
        );
    }

    #[test]
    fn nested_parenthesis_are_evaluated_correctly() {
        let data = mock_follow_data(&[], &[]);

        let expression = parse_expression("1 + ((2 * (4 + 6)) * (3 - 5))").unwrap();

        assert_eq!(
            evaluate_expression(&expression, &data).unwrap(),
            Variable::Int(-39),
        );
    }

    #[test]
    fn strings_can_be_inside_expressions() {
        let data = mock_follow_data(&[], &[]);

        let expression = parse_expression("\"str\" + \"ing\"").unwrap();

        assert_eq!(
            evaluate_expression(&expression, &data).unwrap(),
            Variable::String("string".to_string())
        );
    }

    #[test]
    fn reading_next_operation_string_excludes_markers_inside_strings() {
        let mut buffer = "\"string-with-+-*/%-markers\" + \"ta+-il\"".to_string();

        assert_eq!(
            read_next_operation_string(&mut buffer).unwrap(),
            "\"string-with-+-*/%-markers\" "
        );

        assert_eq!(
            read_next_operation_string(&mut buffer).unwrap(),
            "+ \"ta+-il\""
        );
    }

    #[test]
    fn unmatched_quotation_marks_return_entire_strings_from_last_operator() {
        let mut buffer = "\"unmatched string".to_string();

        assert_eq!(
            read_next_operation_string(&mut buffer).unwrap(),
            "\"unmatched string"
        );

        let mut buffer = "unmatched string\"".to_string();

        assert_eq!(
            read_next_operation_string(&mut buffer).unwrap(),
            "unmatched string\""
        );
    }
}
