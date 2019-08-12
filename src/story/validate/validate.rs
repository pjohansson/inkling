//! Validate story and variable names, addresses, expressions, and conditions.

use crate::{
    error::{parse::validate::ValidationError, utils::MetaData},
    follow::FollowData,
    knot::{get_empty_knot_counts, Address, AddressKind, KnotSet},
    story::{types::VariableSet, validate::namespace::validate_story_name_spaces},
};

use std::collections::HashMap;

pub struct ValidationData {
    /// Data required to evaluate expressions.
    ///
    /// Should be a clone of the original data object, containing all the global variables
    /// and empty knot counts directly after parsing the story structure. The trait may evaluate
    /// variable assignments by trying them out in all parts of the story.
    pub follow_data: FollowData,
    /// Structure corresponding to knots with their default stitch, stitches and meta data.
    pub knots: HashMap<String, KnotValidationInfo>,
}

/// Basic information about a knot, required to validate its content.
pub struct KnotValidationInfo {
    pub default_stitch: String,
    pub stitches: HashMap<String, StitchValidationInfo>,
    pub meta_data: MetaData,
}

/// Basic information about a stitch, required to validate its content.
pub struct StitchValidationInfo {
    pub meta_data: MetaData,
}

impl ValidationData {
    pub fn from_data(knots: &KnotSet, variables: &VariableSet) -> Self {
        let knot_info = knots
            .iter()
            .map(|(knot_name, knot)| {
                let stitches = knot
                    .stitches
                    .iter()
                    .map(|(stitch_name, stitch_data)| {
                        (
                            stitch_name.to_string(),
                            StitchValidationInfo {
                                meta_data: stitch_data.meta_data.clone(),
                            },
                        )
                    })
                    .collect();

                let info = KnotValidationInfo {
                    default_stitch: knot.default_stitch.clone(),
                    stitches,
                    meta_data: knot.meta_data.clone(),
                };

                (knot_name.clone(), info)
            })
            .collect();

        let follow_data = FollowData {
            knot_visit_counts: get_empty_knot_counts(knots),
            variables: variables.clone(),
        };

        ValidationData {
            follow_data,
            knots: knot_info,
        }
    }
}

/// Trait for nesting into all parts of a story and validating elements.
///
/// Elements which will be validated:
///
/// *   Addresses, which should point to locations (possibly with internal shorthand in knots)
///     or global variables
/// *   Expressions, which should contain matching variable types
/// *   Conditions, which should also contain matching variable types in comparisons
/// *   (If implemented) Variable assignments from other variables or expressions
///
/// Should be implemented for all types that touch the content of a constructed story.
/// This will be most if not all line elements: the criteria is if they contain parts which
/// need to be validated or nest other parts of a line that may. For example, lines contain
/// expressions which need to validated, as well as conditions which contain variables and
/// expressions which also need to be validated, and so on.
///
/// All encountered errors will be recorded in the error container but not break the nested
/// search since we want to collect all possible errors at once. To assert whether an error
/// was found we simply check whether this container is empty or not. For this use case this
/// is easier than returning a `Result`.
///
/// The `MetaData` struct is forwarded from the deepest currently active object with such an
/// item, to trace from which line an encountered error stems from. Similarly the `Address`
/// object contains the current location in the story, to be used when checking for internal
/// addressing within knot or stitch name spaces.
///
/// # Notes
/// *   Addresses are validated first, since variables need verified addresses to access
///     underlying content in expressions.
pub trait ValidateContent {
    fn validate(
        &mut self,
        errors: &mut ValidationError,
        current_location: &Address,
        current_meta_data: &MetaData,
        follow_data: &ValidationData,
    );
}

pub fn validate_story_content(
    knots: &mut KnotSet,
    follow_data: &FollowData,
) -> Result<(), ValidationError> {
    let validation_data = ValidationData::from_data(knots, &follow_data.variables);

    let mut error = ValidationError {
        invalid_address_errors: Vec::new(),
        name_space_errors: Vec::new(),
    };

    knots.iter_mut().for_each(|(knot_name, knot)| {
        knot.stitches.iter_mut().for_each(|(stitch_name, stitch)| {
            let current_location = Address::Validated(AddressKind::Location {
                knot: knot_name.clone(),
                stitch: stitch_name.clone(),
            });

            stitch.root.validate(
                &mut error,
                &current_location,
                &stitch.meta_data,
                &validation_data,
            );
        })
    });

    if error.is_empty() {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    use crate::{
        knot::{Knot, Stitch},
        line::Variable,
        node::RootNodeBuilder,
        story::{
            parse::read_story_content_from_string,
            types::{VariableInfo, VariableSet},
        },
    };

    pub fn construct_knots(data: &[(&str, &[&str])]) -> KnotSet {
        let mut line_index = 0;

        data.into_iter()
            .map(|(knot_name, knot_data)| {
                let default_stitch = knot_data[0].to_string();

                let knot_line_index = line_index;
                line_index += 1;

                let stitches = knot_data
                    .into_iter()
                    .map(|stitch_name| {
                        let root = RootNodeBuilder::from_address(knot_name, stitch_name).build();

                        let stitch = Stitch {
                            root,
                            stack: Vec::new(),
                            meta_data: line_index.into(),
                        };

                        line_index += 1;

                        (stitch_name.to_string(), stitch)
                    })
                    .collect();

                let knot = Knot {
                    default_stitch,
                    stitches,
                    tags: Vec::new(),
                    meta_data: knot_line_index.into(),
                };

                (knot_name.to_string(), knot)
            })
            .collect()
    }

    pub fn construct_variables<T>(data: &[(&str, T)]) -> VariableSet
    where
        T: Into<Variable> + Clone,
    {
        data.into_iter()
            .cloned()
            .enumerate()
            .map(|(i, (name, variable))| (name.to_string(), VariableInfo::new(variable.into(), i)))
            .collect()
    }

    fn get_validation_data_from_string(content: &str) -> (KnotSet, FollowData) {
        let (knots, variables, _) = read_story_content_from_string(content).unwrap();

        let data = FollowData {
            knot_visit_counts: get_empty_knot_counts(&knots),
            variables,
        };

        (knots, data)
    }

    fn get_validation_result_from_string(content: &str) -> Result<(), ValidationError> {
        let (mut knots, data) = get_validation_data_from_string(content);
        validate_story_content(&mut knots, &data)
    }

    fn get_validation_error_from_string(content: &str) -> ValidationError {
        let (mut knots, data) = get_validation_data_from_string(content);
        validate_story_content(&mut knots, &data).unwrap_err()
    }

    #[test]
    fn validating_story_raises_error_if_expression_has_non_matching_types() {
        let content = "

{2 + \"string\"}
{true + 1}

";
        let error = get_validation_error_from_string(content);
        panic!();
    }

    #[test]
    fn validating_story_raises_error_if_condition_has_invalid_types_in_comparison() {
        let content = "

{2 + \"string\" == 0: True | False}
*   {true and 2 + \"string\" == 0} Choice

";
        let error = get_validation_error_from_string(content);
        panic!();
    }

    #[test]
    fn validating_story_raises_error_for_every_address_that_does_not_exist() {
        let content = "

-> address
{variable}

";
        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 2);
    }

    #[test]
    fn validating_story_raises_error_for_bad_addresses_in_choices() {
        let content = "

*   {variable == 0} Choice 1
*   Choice 2 -> address
    -> address

";
        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 3);
    }

    #[test]
    fn validating_story_does_not_raise_an_error_for_internal_addressing_in_stitches_and_knots() {
        let content = "

== knot
= one 
-> two

= two
-> one

";

        assert!(get_validation_result_from_string(content).is_ok());
    }

    #[test]
    fn validating_story_raises_an_error_if_addresses_refer_to_internal_addresses_in_other_knots() {
        let content = "

== knot_one
= one 
Line one.

== knot_two
-> one

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 1);
    }

    #[test]
    fn validating_story_sets_all_addresses_to_validated_addresses() {
        let content = "

VAR variable = true

-> knot

== knot
{variable: True | False}

";

        let (mut knots, data) = get_validation_data_from_string(content);

        let pre_validated_addresses = format!("{:?}", &knots).matches("Validated(").count();
        let pre_raw_addresses = format!("{:?}", &knots).matches("Raw(").count();

        assert!(pre_raw_addresses >= 2);

        validate_story_content(&mut knots, &data).unwrap();

        let validated_addresses = format!("{:?}", &knots).matches("Validated(").count();
        let raw_addresses = format!("{:?}", &knots).matches("Raw(").count();

        assert_eq!(raw_addresses, 0);
        assert_eq!(validated_addresses, pre_validated_addresses + 2);
    }
}
