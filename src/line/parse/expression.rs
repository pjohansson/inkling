use crate::{
    error::{LineErrorKind, LineParsingError},
    line::{
        expression::{Operand, Operation},
        Expression, Variable,
    },
};

pub fn parse_expression(content: &str) -> Result<Expression, LineParsingError> {
    unimplemented!();
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let expression = parse_expression("1 + 2 - 2 * 3 / 5").unwrap();
        let equiv_expression = parse_expression("(1 + 2) - ((2 * 3) / 5)").unwrap();

        assert_eq!(expression, equiv_expression);
    }
}
