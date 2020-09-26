//! Trait and functions to validate a story.

use crate::{
    error::{parse::validate::ValidationError, utils::MetaData},
    follow::FollowData,
    knot::{get_empty_knot_counts, Address, AddressKind, KnotSet},
    story::{
        log::Logger, rng::StoryRng, types::VariableSet,
        validate::namespace::validate_story_name_spaces,
    },
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
    /// Default stitch of knot.
    pub default_stitch: String,
    /// Collection of validation data for stitches.
    ///
    /// The keys are the stitch names.
    pub stitches: HashMap<String, StitchValidationInfo>,
    /// Information about the origin of this knot.
    pub meta_data: MetaData,
}

/// Basic information about a stitch, required to validate its content.
pub struct StitchValidationInfo {
    /// Information about the origin of this stitch.
    pub meta_data: MetaData,
}

impl ValidationData {
    /// Construct the required validation data from the parsed knots and variables.
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
            rng: StoryRng::default(),
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
/// *   Conditions, which should also contain matching variable types on each side of a comparison
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
        log: &mut Logger,
        current_location: &Address,
        current_meta_data: &MetaData,
        follow_data: &ValidationData,
    );
}

/// Validate addresses, expressions, conditions and names of all content in a story.
///
/// This function walks through all the knots and stitches in a story, and for each item
/// uses the `ValidateContent` trait to nest through its content. Additionally it checks for
/// name space collisions between variables, knots and stitches.
///
/// If any error is encountered this will yield the set of all found errors.
pub fn validate_story_content(
    knots: &mut KnotSet,
    follow_data: &FollowData,
    log: &mut Logger,
) -> Result<(), ValidationError> {
    let validation_data = ValidationData::from_data(knots, &follow_data.variables);

    let mut error = ValidationError::new();

    knots.iter_mut().for_each(|(knot_name, knot)| {
        knot.stitches.iter_mut().for_each(|(stitch_name, stitch)| {
            let current_location = Address::Validated(AddressKind::Location {
                knot: knot_name.clone(),
                stitch: stitch_name.clone(),
            });

            stitch.root.validate(
                &mut error,
                log,
                &current_location,
                &stitch.meta_data,
                &validation_data,
            );
        })
    });

    if let Err(name_space_errors) = validate_story_name_spaces(&validation_data) {
        error.name_space_errors = name_space_errors;
    }

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
        consts::ROOT_KNOT_NAME,
        follow::FollowDataBuilder,
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
        let mut log = Logger::default();
        let (knots, variables, _) = read_story_content_from_string(content, &mut log).unwrap();

        let data = FollowDataBuilder::new()
            .with_knots(get_empty_knot_counts(&knots))
            .with_variables(variables)
            .build();

        (knots, data)
    }

    fn get_validation_result_from_string(content: &str) -> Result<(), ValidationError> {
        let (mut knots, data) = get_validation_data_from_string(content);
        let mut log = Logger::default();

        validate_story_content(&mut knots, &data, &mut log)
    }

    fn get_validation_error_from_string(content: &str) -> ValidationError {
        let (mut knots, data) = get_validation_data_from_string(content);
        let mut log = Logger::default();

        validate_story_content(&mut knots, &data, &mut log).unwrap_err()
    }

    #[test]
    fn creating_validation_data_sets_default_knot_names() {
        let content = "
== tripoli
= cinema
-> END
= with_family
-> END

== addis_ababa
-> END
= with_family
-> END
";

        let mut log = Logger::default();
        let (knots, _, _) = read_story_content_from_string(content, &mut log).unwrap();

        let data = ValidationData::from_data(&knots, &HashMap::new());

        assert_eq!(data.knots.len(), 3);

        let tripoli_default = &data.knots.get("tripoli").unwrap().default_stitch;
        let addis_ababa_default = &data.knots.get("addis_ababa").unwrap().default_stitch;

        assert_eq!(tripoli_default, "cinema");
        assert_eq!(addis_ababa_default, ROOT_KNOT_NAME);
    }

    #[test]
    fn creating_validation_data_sets_stitches() {
        let content = "
== tripoli
= cinema
-> END
= with_family
-> END

== addis_ababa
-> END
= with_family
-> END
";

        let mut log = Logger::default();
        let (knots, _, _) = read_story_content_from_string(content, &mut log).unwrap();

        let data = ValidationData::from_data(&knots, &HashMap::new());

        let tripoli_stitches = &data.knots.get("tripoli").unwrap().stitches;
        let addis_ababa_stitches = &data.knots.get("addis_ababa").unwrap().stitches;

        assert_eq!(tripoli_stitches.len(), 2);
        assert!(tripoli_stitches.contains_key(&"cinema".to_string()));
        assert!(tripoli_stitches.contains_key(&"with_family".to_string()));

        assert_eq!(addis_ababa_stitches.len(), 2);
        assert!(addis_ababa_stitches.contains_key(&ROOT_KNOT_NAME.to_string()));
        assert!(addis_ababa_stitches.contains_key(&"with_family".to_string()));
    }

    #[test]
    fn creating_validation_data_sets_variable_names() {
        let mut variables = HashMap::new();

        variables.insert("counter".to_string(), VariableInfo::new(1, 0));
        variables.insert("health".to_string(), VariableInfo::new(75.0, 1));

        let data = ValidationData::from_data(&HashMap::new(), &variables);

        assert_eq!(data.follow_data.variables.len(), 2);
        assert!(data.follow_data.variables.contains_key("counter"));
        assert!(data.follow_data.variables.contains_key("health"));
    }

    #[test]
    fn validating_story_raises_error_if_expression_has_non_matching_types() {
        let content = "

{2 + \"string\"}
{true + 1}

";
        let error = get_validation_error_from_string(content);

        assert_eq!(error.variable_errors.len(), 2);
    }

    #[test]
    fn validating_story_raises_error_if_condition_has_invalid_types_in_comparison() {
        let content = "

{2 + \"string\" == 0: True | False}
*   {true and 3 + \"string\" == 0} Choice

";
        let error = get_validation_error_from_string(content);

        assert_eq!(error.variable_errors.len(), 2);
    }

    #[test]
    fn validating_story_raises_error_if_comparison_is_between_different_types() {
        let content = "

VAR int = 0

{\"string\" == 0: True | False}
{0 == \"string\": True | False}
{0 == \"string\": True | False}
{int == \"string\": True | False}
{0 == true: True | False}

";
        let error = get_validation_error_from_string(content);

        assert_eq!(error.variable_errors.len(), 5);
    }

    #[test]
    fn all_expressions_in_conditions_are_validated() {
        let content = "

{true and 2 + \"str\" == 0 or 3 + true == 0: True | False}

";
        let error = get_validation_error_from_string(content);

        assert_eq!(error.variable_errors.len(), 2);
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
        let mut log = Logger::default();

        let pre_validated_addresses = format!("{:?}", &knots).matches("Validated(").count();
        let pre_raw_addresses = format!("{:?}", &knots).matches("Raw(").count();

        assert!(pre_raw_addresses >= 2);

        validate_story_content(&mut knots, &data, &mut log).unwrap();

        let validated_addresses = format!("{:?}", &knots).matches("Validated(").count();
        let raw_addresses = format!("{:?}", &knots).matches("Raw(").count();

        assert_eq!(raw_addresses, 0);
        assert_eq!(validated_addresses, pre_validated_addresses + 2);
    }

    #[test]
    fn encountered_invalid_address_errors_stop_expressions_from_trying_to_evaluate() {
        let content = "

{knot + \"string\"}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 1);
        assert!(error.variable_errors.is_empty());
    }

    #[test]
    fn encountered_invalid_address_errors_stop_conditions_from_trying_to_evaluate() {
        let content = "

{knot + \"string\" == 0: True | False}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 1);
        assert!(error.variable_errors.is_empty());
    }

    #[test]
    fn invalid_addresses_in_choices_can_be_in_selection_text_only() {
        let content = "

*   Invalid address in selection text: [{knot}]

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 1);
        assert!(error.variable_errors.is_empty());
    }

    #[test]
    fn invalid_addresses_in_choices_can_be_in_display_text_only() {
        let content = "

*   Invalid address in display text: [] {knot}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 1);
        assert!(error.variable_errors.is_empty());
    }

    #[test]
    fn address_validation_is_done_in_first_displayed_text_of_branching_choice() {
        let content = "

*   Invalid address in same line display text: [] {knot}
*   [Selection]
    Invalid address in next line display text: {knot}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 2);
        assert!(error.variable_errors.is_empty());
    }

    #[test]
    fn expression_validation_is_done_in_first_displayed_text_of_branching_choice() {
        let content = "

*   Invalid expression in same line display text: [] {2 + \"string\"}
*   [Selection]
    Invalid expression in next line display text: {2 + \"string\"}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.variable_errors.len(), 2);
    }

    #[test]
    fn addresses_are_validated_if_correct_in_all_displayed_text_of_branching_choices() {
        let content = "

VAR variable = 0

*   \\{variable}
*   [Selection] -> knot
*   [Selection 2]
    -> knot

== knot
Line

";

        let (mut knots, data) = get_validation_data_from_string(content);
        let mut log = Logger::default();

        let pre_raw_addresses = format!("{:?}", &knots).matches("Raw(").count();

        assert!(pre_raw_addresses >= 3);

        validate_story_content(&mut knots, &data, &mut log).unwrap();

        dbg!(&knots);

        let raw_addresses = format!("{:?}", &knots).matches("Raw(").count();

        assert_eq!(raw_addresses, 0);
    }

    #[test]
    fn invalid_address_errors_in_choices_with_display_and_selection_text_validates_expr_once() {
        let content = "

*   {knot} Choice with an invalid address in condition
*   Choice with an invalid address in an expression: {knot}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.invalid_address_errors.len(), 2);
        assert!(error.variable_errors.is_empty());
    }

    #[test]
    fn items_inside_true_parts_of_conditions_are_validated() {
        let content = "

{true: {knot}}
{true: {2 + \"string\"}}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.num_errors(), 2);
    }

    #[test]
    fn items_inside_false_parts_of_conditions_are_validated() {
        let content = "

{true: True | {knot}}
{true: True | {2 + \"string\"}}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.num_errors(), 2);
    }

    #[test]
    fn items_inside_parts_of_alternative_sequences_are_validated() {
        let content = "

{{2 + \"string\"} | {knot} | -> other_knot}

";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.num_errors(), 3);
    }

    #[test]
    fn expressions_add_one_error_for_errors_in_nested_parts() {
        let content = "{1 + (2 + (3 + true))}";

        let error = get_validation_error_from_string(content);

        assert_eq!(error.variable_errors.len(), 1);
    }
}
