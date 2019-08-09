//! Parse `Variable` objects.

use crate::{
    consts::DIVERT_MARKER,
    error::parse::variable::{VariableError, VariableErrorKind},
    knot::Address,
    line::{parse::validate_address, Variable},
};

/// Parse a `Variable` from a line.
pub fn parse_variable(content: &str) -> Result<Variable, VariableError> {
    let content = content.trim();

    if content.to_lowercase() == "true" {
        Ok(Variable::Bool(true))
    } else if content.to_lowercase() == "false" {
        Ok(Variable::Bool(false))
    } else if content.starts_with('"') && content.ends_with('"') && content.len() > 2 {
        Ok(Variable::String(
            content.get(1..content.len() - 1).unwrap().to_string(),
        ))
    } else if content.starts_with(DIVERT_MARKER) {
        let inner = content.get(DIVERT_MARKER.len()..).unwrap().trim();

        validate_address(inner)
            .map(|address| Variable::Divert(Address::Raw(address)))
            .map_err(|_| VariableErrorKind::InvalidDivert {
                address: inner.to_string(),
            })
    } else if content.starts_with(|c: char| c.is_numeric() || c == '-' || c == '+') {
        parse_number(content)
    } else {
        validate_address(content.trim())
            .map(|address| Variable::Address(Address::Raw(address)))
            .map_err(|_| VariableErrorKind::InvalidAddress)
    }
    .map_err(|kind| VariableError {
        content: content.to_string(),
        kind,
    })
}

/// Parse a variable number from a string.
fn parse_number(content: &str) -> Result<Variable, VariableErrorKind> {
    if content.contains('.') {
        content
            .parse::<f32>()
            .map(|value| Variable::Float(value))
            .map_err(|err| VariableErrorKind::InvalidNumericValue { err: Box::new(err) })
    } else {
        content
            .parse::<i32>()
            .map(|value| Variable::Int(value))
            .map_err(|err| VariableErrorKind::InvalidNumericValue { err: Box::new(err) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_integer_numbers_as_regular_numbers() {
        assert_eq!(parse_variable("5").unwrap(), Variable::Int(5));
        assert_eq!(parse_variable("-5").unwrap(), Variable::Int(-5));
        assert_eq!(parse_variable("+5").unwrap(), Variable::Int(5));
    }

    #[test]
    fn parse_floating_point_numbers_as_numbers_with_decimals() {
        assert_eq!(parse_variable("3.0").unwrap(), Variable::Float(3.0));
        assert_eq!(parse_variable("3.").unwrap(), Variable::Float(3.0));
        assert_eq!(parse_variable("3.3").unwrap(), Variable::Float(3.3));
        assert_eq!(parse_variable("-3.3").unwrap(), Variable::Float(-3.3));
        assert_eq!(parse_variable("+3.3").unwrap(), Variable::Float(3.3));
    }

    #[test]
    fn parse_diverts_from_starting_divert_marker() {
        assert_eq!(
            parse_variable("-> root").unwrap(),
            Variable::Divert(Address::Raw("root".to_string()))
        );
    }

    #[test]
    fn diverts_must_have_valid_addresses() {
        assert!(parse_variable("->").is_err());
        assert!(parse_variable("-> ").is_err());
        assert!(parse_variable("-> two words").is_err());
        assert!(parse_variable("-> two$words").is_err());
    }

    #[test]
    fn parse_booleans_as_exact_string_matches() {
        assert_eq!(parse_variable("true").unwrap(), Variable::Bool(true));
        assert_eq!(parse_variable("false").unwrap(), Variable::Bool(false));
        assert_eq!(parse_variable("TRUE").unwrap(), Variable::Bool(true));
        assert_eq!(parse_variable("FALSE").unwrap(), Variable::Bool(false));
    }

    #[test]
    fn parse_single_words_as_raw_addresses() {
        assert_eq!(
            parse_variable("knot").unwrap(),
            Variable::Address(Address::Raw("knot".to_string()))
        );
    }

    #[test]
    fn multiple_words_are_invalid() {
        assert!(parse_variable("knot other_knot").is_err());
    }

    #[test]
    fn parse_string_variable_within_quotation_marks() {
        assert_eq!(
            parse_variable("\"word\"").unwrap(),
            Variable::String("word".to_string())
        );

        assert_eq!(
            parse_variable("\"two words\"").unwrap(),
            Variable::String("two words".to_string())
        );
    }

    #[test]
    fn parsing_single_quotation_mark_string_is_error() {
        assert!(parse_variable("\"").is_err());
        assert!(parse_variable("\"word").is_err());
        assert!(parse_variable("word\"").is_err());
    }

    #[test]
    fn whitespace_is_trimmed_before_parsing() {
        assert_eq!(
            parse_variable("   3.55   ").unwrap(),
            parse_variable("3.55").unwrap()
        );
        assert_eq!(
            parse_variable("   true   ").unwrap(),
            parse_variable("true").unwrap()
        );
    }
}
