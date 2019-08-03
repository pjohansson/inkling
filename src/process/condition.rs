//! Checking of `Condition`s which determine whether content will be displayed.

use crate::{
    error::InklingError,
    follow::FollowData,
    knot::get_num_visited,
    line::{Condition, StoryCondition},
};

/// Check whether a single condition is fulfilled.
pub fn check_condition(condition: &Condition, data: &FollowData) -> Result<bool, InklingError> {
    let evaluator = |kind: &StoryCondition| match kind {
        StoryCondition::NumVisits {
            address,
            rhs_value,
            ordering,
        } => {
            let num_visited = get_num_visited(address, data)? as i32;

            Ok(num_visited.cmp(rhs_value) == *ordering)
        }
    };

    condition.evaluate(&evaluator)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{consts::ROOT_KNOT_NAME, knot::Address, line::ConditionBuilder};

    use std::{cmp::Ordering, collections::HashMap};

    fn mock_data_with_single_stitch(knot: &str, stitch: &str, num_visited: u32) -> FollowData {
        let mut stitch_count = HashMap::new();
        stitch_count.insert(stitch.to_string(), num_visited);

        let mut knot_visit_counts = HashMap::new();
        knot_visit_counts.insert(knot.to_string(), stitch_count);

        FollowData {
            knot_visit_counts,
            variables: HashMap::new(),
        }
    }

    #[test]
    fn check_some_conditions_against_number_of_visits_in_a_hash_map() {
        let name = "knot_name".to_string();

        let data = mock_data_with_single_stitch(&name, ROOT_KNOT_NAME, 3);

        let address = Address::Validated {
            knot: name.clone(),
            stitch: ROOT_KNOT_NAME.to_string(),
        };

        let greater_than_condition = StoryCondition::NumVisits {
            address: address.clone(),
            rhs_value: 2,
            ordering: Ordering::Greater,
        };

        let less_than_condition = StoryCondition::NumVisits {
            address: address.clone(),
            rhs_value: 2,
            ordering: Ordering::Less,
        };

        let equal_condition = StoryCondition::NumVisits {
            address: address.clone(),
            rhs_value: 3,
            ordering: Ordering::Equal,
        };

        let not_equal_condition = StoryCondition::NumVisits {
            address: address.clone(),
            rhs_value: 3,
            ordering: Ordering::Equal,
        };

        let gt_condition =
            ConditionBuilder::from_kind(&greater_than_condition.into(), false).build();
        let lt_condition = ConditionBuilder::from_kind(&less_than_condition.into(), false).build();
        let eq_condition = ConditionBuilder::from_kind(&equal_condition.into(), false).build();
        let neq_condition = ConditionBuilder::from_kind(&not_equal_condition.into(), true).build();

        assert!(check_condition(&gt_condition, &data).unwrap());
        assert!(!check_condition(&lt_condition, &data).unwrap());
        assert!(check_condition(&eq_condition, &data).unwrap());
        assert!(!check_condition(&neq_condition, &data).unwrap());
    }
}
