//! Validate story and variable names, addresses, expressions, and conditions.

use crate::{
    error::{
        parse::validate::{CollisionKind, NameSpaceCollision, ValidationError},
        utils::MetaData,
    },
    follow::FollowData,
    knot::{Address, KnotSet},
};

use std::collections::HashMap;

pub struct ValidationData {
    /// Data required to evaluate expressions.
    ///
    /// Should be a clone of the original data object, containing all the global variables
    /// and empty knot counts directly after parsing the story structure. The trait may evaluate
    /// variable assignments by trying them out in all parts of the story.
    pub follow_data: FollowData,
    /// Structure corresponding to knots with their stitches and default stitch.
    pub knot_structure: HashMap<String, (String, Vec<String>)>,
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
    fn validate2(
        &mut self,
        errors: &mut ValidationError,
        current_location: &Address,
        current_meta_data: &MetaData,
        data: &ValidationData,
    );
}

pub fn validate_story_content(
    knots: &mut KnotSet,
    data: &FollowData,
) -> Result<(), ValidationError> {
    unimplemented!();
}

/// Validate that there are no name space collisions in the story addresses.
///
/// Elements which will be validated:
///
/// *   Namespace collisions from stitches to knots and variables
/// *   Namespace collisions from variables to knots
/// *   (If implemented) Namespace collisions from labels to stitches, knots and variables
///
/// All name space collisions will be recorded in the returned error.
pub fn validate_story_name_spaces(data: &ValidationData) -> Result<(), Vec<NameSpaceCollision>> {
    unimplemented!();
}
