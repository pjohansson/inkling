//! Check story structures for name space collisions.

use crate::{
    error::{
        parse::validate::{CollisionKind, NameSpaceCollision},
        utils::MetaData,
    },
    story::{
        types::VariableInfo,
        validate::validate::{KnotValidationInfo, StitchValidationInfo, ValidationData},
    },
};

/// Trait to easily construct a `NameSpaceCollision` error.
trait NameSpaceCollisionData {
    const KIND: CollisionKind;

    fn get_meta_data(&self) -> &MetaData;
}

impl NameSpaceCollisionData for KnotValidationInfo {
    const KIND: CollisionKind = CollisionKind::Knot;

    fn get_meta_data(&self) -> &MetaData {
        &self.meta_data
    }
}

impl NameSpaceCollisionData for StitchValidationInfo {
    const KIND: CollisionKind = CollisionKind::Stitch;

    fn get_meta_data(&self) -> &MetaData {
        &self.meta_data
    }
}

impl NameSpaceCollisionData for VariableInfo {
    const KIND: CollisionKind = CollisionKind::Variable;

    fn get_meta_data(&self) -> &MetaData {
        &self.meta_data
    }
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
    let mut errors = Vec::new();

    for (name, variable_info) in &data.follow_data.variables {
        if let Some(knot_info) = data.knots.get(name) {
            errors.push(get_collision_error(name, variable_info, knot_info));
        }
    }

    for knot_info in data.knots.values() {
        for (stitch_name, stitch_info) in &knot_info.stitches {
            if let Some(knot_info) = &data.knots.get(stitch_name) {
                errors.push(get_collision_error(stitch_name, stitch_info, *knot_info));
            }

            if let Some(variable_info) = &data.follow_data.variables.get(stitch_name) {
                errors.push(get_collision_error(
                    stitch_name,
                    stitch_info,
                    *variable_info,
                ));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Construct a `NameSpaceCollision` error from the given types.
fn get_collision_error<F, T>(name: &str, from: &F, to: &T) -> NameSpaceCollision
where
    F: NameSpaceCollisionData,
    T: NameSpaceCollisionData,
{
    NameSpaceCollision {
        name: name.to_string(),
        from_kind: F::KIND,
        from_meta_data: from.get_meta_data().clone(),
        to_kind: T::KIND,
        to_meta_data: to.get_meta_data().clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        knot::KnotSet,
        story::{
            types::VariableSet,
            validate::validate::tests::{construct_knots, construct_variables},
        },
    };

    #[test]
    fn empty_stitch_and_variable_sets_give_no_name_space_errors() {
        let knots = KnotSet::new();
        let variables = VariableSet::new();

        let data = ValidationData::from_data(&knots, &variables);

        assert!(validate_story_name_spaces(&data).is_ok());
    }

    #[test]
    fn knots_stitches_and_variables_with_unique_names_raise_no_name_space_errors() {
        let knots = construct_knots(&[("knot", &[("stitch")])]);
        let variables = construct_variables(&[("variable", 1)]);

        let data = ValidationData::from_data(&knots, &variables);

        assert!(validate_story_name_spaces(&data).is_ok());
    }

    #[test]
    fn stitches_may_share_name_with_stitches_in_other_knots_without_name_space_error() {
        let knots = construct_knots(&[("one", &["stitch"]), ("two", &["stitch"])]);
        let variables = VariableSet::new();

        let data = ValidationData::from_data(&knots, &variables);

        assert!(validate_story_name_spaces(&data).is_ok());
    }

    #[test]
    fn stitch_names_cannot_collide_with_knot_names() {
        let knots = construct_knots(&[("knot", &[("knot")])]);
        let variables = VariableSet::new();

        let data = ValidationData::from_data(&knots, &variables);

        let errors = validate_story_name_spaces(&data).unwrap_err();

        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn stitch_names_cannot_collide_with_variable_names() {
        let knots = construct_knots(&[("knot", &[("variable")])]);
        let variables = construct_variables(&[("variable", 1)]);

        let data = ValidationData::from_data(&knots, &variables);

        let errors = validate_story_name_spaces(&data).unwrap_err();

        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn variable_names_cannot_collide_with_knot_names() {
        let knots = construct_knots(&[("knot", &[("stitch")])]);
        let variables = construct_variables(&[("knot", 1)]);

        let data = ValidationData::from_data(&knots, &variables);

        let errors = validate_story_name_spaces(&data).unwrap_err();

        assert_eq!(errors.len(), 1);
    }
}
