//! Parse `Expression` objects.

use crate::{
    error::{
        parse::{ExpressionError, ExpressionErrorKind},
        LineError,
    },
    line::{
        expression::{apply_order_of_operations, Operand, Operator},
        parse::{parse_variable, split_line_at_separator_parenthesis},
        Expression,
    },
};

/// List of valid mathematical operators.
pub const MATHEMATICAL_OPERATORS: &[char] = &['+', '-', '*', '/', '%'];

/// Parse a mathematical `Expression` from a string.
///
/// The expression may be a numerical expression or string concatenation.
///
/// Numerical expressions may use the standard mathematical operators and parenthesis.
/// Terms within parenthesis will be grouped together into single units, and order
/// of operator precedence is applied to group multiplication, division and remainder
/// operations together before addition and subtraction.
///
/// String concatenation should only use addition.
pub fn parse_expression(content: &str) -> Result<Expression, ExpressionError> {
    split_line_into_operation_terms(content)
        .and_then(|operations| parse_expression_from_operation_terms(operations))
        .map(|expression| apply_order_of_operations(&expression))
        .map_err(|kind| ExpressionError {
            content: content.to_string(),
            kind,
        })
}

/// Parse a list of operation terms into a single `Expression`.
///
/// If the list is empty, return an `ExpressionErrorKind::Empty` error. If it is a single
/// term, that term will be the head of the expression. Additional terms are added along
/// with their leading operators to the expression tail.
///
/// The head may be preceeded by either `+` or `-` (ie. be on the form `+a` or `-a`).
/// A minus will add a negative multiplier after the head, which takes care for the negation.
///
/// # Notes
/// *   The list is split into a head and tail. This function wants all strings in the tail
///     to lead with a mathematical operator (be on the form `+ a` etc.), which is always
///     the case from a call to `split_line_into_operation_terms`.
fn parse_expression_from_operation_terms(
    mut operation_strings: Vec<String>,
) -> Result<Expression, ExpressionErrorKind> {
    operation_strings
        .split_first_mut()
        .ok_or(ExpressionErrorKind::Empty)
        .and_then(|(head_string, tail_strings)| {
            let (head, head_multiplier) = get_head_operand_and_multiplier(head_string)?;

            let mut tail = tail_strings
                .into_iter()
                .map(|content| get_tail_operator_and_operand(content))
                .collect::<Result<Vec<_>, _>>()?;

            if let Some(multiplier) = head_multiplier {
                tail.insert(0, multiplier);
            }

            Ok(Expression { head, tail })
        })
}

/// Parse the head term for its value and possible negation.
///
/// The negation comes from terms on the form `-a` or similar. For this case we return
/// the head operand along with a `* -1` multiplier to negate it.
///
/// Terms on the form `+a` just resolve into `a` while terms with other leading operators
/// like `*`, `/` and `%` yield and error since they cannot be applied without a left hand
/// side value.
fn get_head_operand_and_multiplier(
    content: &str,
) -> Result<(Operand, Option<(Operator, Operand)>), ExpressionErrorKind> {
    let mut buffer = content.to_string();

    let multiplier = match split_off_operator(&mut buffer) {
        Some(Operator::Subtract) => {
            let operand = Operand::Variable((-1).into());
            Ok(Some((Operator::Multiply, operand)))
        }
        Some(Operator::Add) | None => Ok(None),
        _ => Err(ExpressionErrorKind::InvalidHead {
            head: content.to_string(),
        }),
    }?;

    let head = parse_operand(buffer.trim())?;

    Ok((head, multiplier))
}

/// Parse a tail term for its leading operator and operand.
fn get_tail_operator_and_operand(
    content: &mut String,
) -> Result<(Operator, Operand), ExpressionErrorKind> {
    let operator = split_off_operator(content).ok_or(ExpressionErrorKind::NoOperator {
        content: content.to_string(),
    })?;

    let operand = parse_operand(content.trim())?;

    Ok((operator, operand))
}

/// Split a line with a mathematical expression in text into its terms.
///
/// For `n` operations this returns `n + 1` terms, where the first in the returned string
/// is the single root value without an operator (a '+' is implied), and the remaining
/// `n` terms contain the operator and operand.
///
/// For the expression `a + b * (c + d) - e` this returns `["a ", "+ b ", "* (c + d) ", "- e"]`.
/// For the expression `a + "one-term" - b` it returns `["a ", "+ \"one-term\" ", "- b"].
fn split_line_into_operation_terms(content: &str) -> Result<Vec<String>, ExpressionErrorKind> {
    let mut buffer = content.trim().to_string();
    let mut operations = Vec::new();

    while !buffer.trim().is_empty() {
        let operation_string = read_next_operation_string(&mut buffer)?;

        operations.push(operation_string);
    }

    Ok(operations)
}

/// Parse the `Operand` from an expression.
///
/// Assumes that the given string is trimmed of whitespace from both ends.
fn parse_operand(content: &str) -> Result<Operand, ExpressionErrorKind> {
    if content.starts_with('(') && content.ends_with(')') && content.len() > 1 {
        let inner = content.get(1..content.bytes().len() - 1).unwrap();

        parse_expression(inner)
            .map(|expression| Operand::Nested(Box::new(expression)))
            .map_err(|err| err.kind)
    } else {
        parse_variable(content)
            .map(|variable| Operand::Variable(variable))
            .map_err(|kind| LineError::from_kind(content, kind))
            .map_err(|err| ExpressionErrorKind::InvalidVariable(Box::new(err)))
    }
}

/// Split off the initial operator and return its type.
///
/// Assumes to be called on lines for which operators were definitely found. This should
/// always be the case, since we split the lines where we find the operators.
fn split_off_operator(buffer: &mut String) -> Option<Operator> {
    let operator = buffer.chars().next().and_then(|c| match c {
        '+' => Some(Operator::Add),
        '-' => Some(Operator::Subtract),
        '*' => Some(Operator::Multiply),
        '/' => Some(Operator::Divide),
        '%' => Some(Operator::Remainder),
        _ => None,
    });

    if operator.is_some() {
        buffer.drain(..1);
    }

    operator
}

/// Split the string corresponding to the next whole operation from the buffer.
///
/// Splits occur when mathematical operators '+', '-', '*', '/' and '%' are encountered
/// outside of parenthesis and strings (marked by '""' marks).
///
/// For an input buffer of `a + (b * c) - d` this returns `a `, leaving the buffer as
/// `+ (b * c) - d`. Operating again on the buffer returns `+ (b * c) `, leaving `- d`.
/// A final operation drains the buffer completely and returns `- d`.
fn read_next_operation_string(buffer: &mut String) -> Result<String, ExpressionErrorKind> {
    let (head, tail) = split_leading_operator(&buffer);
    let head_size = head.len();

    let mut last_index = 0;

    let index = loop {
        let haystack = tail.get(last_index..).unwrap();

        let i = get_closest_split_index(haystack)
            .map_err(|_| ExpressionErrorKind::UnmatchedParenthesis)?;

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

/// Trim leading mathematical operator from line.
///
/// Assumes that the given string is trimmed from the start.
fn split_leading_operator(content: &str) -> (&str, &str) {
    let index = if content
        .chars()
        .next()
        .map(|c| MATHEMATICAL_OPERATORS.contains(&c))
        .unwrap_or(false)
    {
        1
    } else {
        0
    };

    content.split_at(index)
}

/// Return the lowest index for any mathematical operator in a line.
fn get_closest_split_index(content: &str) -> Result<usize, LineError> {
    get_split_index(content, "+")
        .and_then(|current_min| get_split_index(&content, "-").map(|next| current_min.min(next)))
        .and_then(|current_min| get_split_index(&content, "*").map(|next| current_min.min(next)))
        .and_then(|current_min| get_split_index(&content, "/").map(|next| current_min.min(next)))
        .and_then(|current_min| get_split_index(&content, "%").map(|next| current_min.min(next)))
}

/// Return the lowest index for the given separator keyword in the line.
fn get_split_index(content: &str, separator: &str) -> Result<usize, LineError> {
    split_line_at_separator_parenthesis(content, separator, Some(1))
        .map(|parts| parts[0].as_bytes().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        follow::FollowData,
        knot::Address,
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
            (Operator::Add, Operand::Variable(Variable::Int(2)))
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
    fn parenthesis_can_nest_several_levels_at_once() {
        let data = mock_follow_data(&[], &[]);

        let expression = parse_expression("((((1 + 2))))").unwrap();

        assert_eq!(
            evaluate_expression(&expression, &data).unwrap(),
            Variable::Int(3),
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
    fn parsing_expression_from_no_terms_yields_empty_error() {
        match parse_expression_from_operation_terms(vec![]) {
            Err(ExpressionErrorKind::Empty) => (),
            other => panic!("expected `ExpressionErrorKind::Empty` but got {:?}", other),
        }
    }

    #[test]
    fn parsing_expression_from_single_term_yields_single_term_expression() {
        let expression = parse_expression_from_operation_terms(vec!["a".to_string()]).unwrap();

        assert_eq!(
            expression.head,
            Operand::Variable(Variable::Address(Address::Raw("a".to_string())))
        );
    }

    #[test]
    fn parsing_expression_from_single_term_with_leading_plus_gives_regular_expression() {
        let expression = parse_expression_from_operation_terms(vec!["+a".to_string()]).unwrap();

        let expression_equiv =
            parse_expression_from_operation_terms(vec!["a".to_string()]).unwrap();

        assert_eq!(expression, expression_equiv);
    }

    #[test]
    fn parsing_expression_from_single_negated_term_creates_multiplication_by_negative_one() {
        let expression =
            parse_expression_from_operation_terms(vec!["-a ".to_string(), "+ 1".to_string()])
                .unwrap();

        let expression_equiv = parse_expression_from_operation_terms(vec![
            "a ".to_string(),
            "* -1".to_string(),
            "+ 1".to_string(),
        ])
        .unwrap();

        assert_eq!(expression, expression_equiv);
    }

    #[test]
    fn parsing_expression_from_single_term_with_leading_mul_div_or_rem_marker_yields_error() {
        match parse_expression_from_operation_terms(vec!["*a".to_string()]) {
            Err(ExpressionErrorKind::InvalidHead { head }) => {
                assert_eq!(head, "*a");
            }
            other => panic!(
                "expected `ExpressionErrorKind::InvalidHead` but got {:?}",
                other
            ),
        }

        match parse_expression_from_operation_terms(vec!["/a".to_string()]) {
            Err(ExpressionErrorKind::InvalidHead { head }) => {
                assert_eq!(head, "/a");
            }
            other => panic!(
                "expected `ExpressionErrorKind::InvalidHead` but got {:?}",
                other
            ),
        }

        match parse_expression_from_operation_terms(vec!["%a".to_string()]) {
            Err(ExpressionErrorKind::InvalidHead { head }) => {
                assert_eq!(head, "%a");
            }
            other => panic!(
                "expected `ExpressionErrorKind::InvalidHead` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn empty_string_splits_into_no_strings() {
        assert!(split_line_into_operation_terms("").unwrap().is_empty());
        assert!(split_line_into_operation_terms("    ").unwrap().is_empty());
    }

    #[test]
    fn string_with_single_term_splits_into_single_term_list() {
        assert_eq!(split_line_into_operation_terms("a").unwrap(), &["a"]);
    }

    #[test]
    fn string_with_pure_number_operations_splits_cleanly_into_parts() {
        assert_eq!(
            split_line_into_operation_terms("a + b * c - d / e + f % g").unwrap(),
            &["a ", "+ b ", "* c ", "- d ", "/ e ", "+ f ", "% g"]
        );
    }

    #[test]
    fn whitespace_is_trimmed_from_ends_when_splitting_into_terms() {
        assert_eq!(
            split_line_into_operation_terms("    a + b    ").unwrap(),
            &["a ", "+ b"]
        );
    }

    #[test]
    fn string_with_parenthesis_as_terms_keep_them_whole() {
        assert_eq!(
            split_line_into_operation_terms("a + (b * (c - d)) / (e + g)").unwrap(),
            &["a ", "+ (b * (c - d)) ", "/ (e + g)"]
        );
    }

    #[test]
    fn whitespace_between_operators_is_not_needed() {
        assert_eq!(
            split_line_into_operation_terms("a+(b*(c-d))/(e+g)").unwrap(),
            &["a", "+(b*(c-d))", "/(e+g)"]
        );
    }

    #[test]
    fn string_that_starts_with_mathematical_operator_returns_the_whole_term_as_first() {
        assert_eq!(
            split_line_into_operation_terms("+ a - b").unwrap(),
            &["+ a ", "- b"]
        );
    }

    #[test]
    fn operators_inside_string_terms_do_not_split() {
        assert_eq!(
            split_line_into_operation_terms("a + \"word-with-dash\" + b").unwrap(),
            &["a ", "+ \"word-with-dash\" ", "+ b"]
        );
    }

    #[test]
    fn variables_may_be_multibyte_characters() {
        assert_eq!(
            split_line_into_operation_terms("a + 한글 / (e + g)").unwrap(),
            &["a ", "+ 한글 ", "/ (e + g)"]
        );
    }

    #[test]
    fn string_terms_may_contain_multibyte_characters_without_affecting_behavior() {
        assert_eq!(
            split_line_into_operation_terms("a + \"word-with-한글\" + b").unwrap(),
            &["a ", "+ \"word-with-한글\" ", "+ b"]
        );
    }

    #[test]
    fn string_terms_can_come_first_and_last() {
        assert_eq!(
            split_line_into_operation_terms("\"one\" + \"two\"").unwrap(),
            &["\"one\" ", "+ \"two\""]
        );
    }

    #[test]
    fn ummatched_string_quotes_keeps_all_content_from_opening_quote_as_one() {
        assert_eq!(
            split_line_into_operation_terms("\"one + \"two\"").unwrap(),
            &["\"one + \"two\""]
        );

        assert_eq!(
            split_line_into_operation_terms("\"one\" + two\"").unwrap(),
            &["\"one\" ", "+ two\""]
        );

        assert_eq!(
            split_line_into_operation_terms("\"one\" + word-with-dash\"").unwrap(),
            &["\"one\" ", "+ word", "-with", "-dash\""]
        );
    }
}
