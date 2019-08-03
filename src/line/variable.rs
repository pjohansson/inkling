//! Types of variables used in a story.

use crate::{
    error::{InklingError, InvalidAddressError},
    follow::FollowData,
    knot::{get_num_visited, Address, AddressKind, ValidateAddressData, ValidateAddresses},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Variables in a story.
///
/// Not all of these will evaluate to a string when used as a variable. Numbers and strings
/// make perfect sense to print: a divert to another location, not as much.
///
/// Variables which cannot be printed will raise errors when used as such.
pub enum Variable {
    /// Address to a stitch or other variable.
    /// 
    /// If the address is another variable in the story it will evaluate to that. If it 
    /// is a location in the story it will evaluate to the number of times it has 
    /// been visited.
    Address(Address),
    /// True or false, evaluates to 1 for true and 0 for false.
    Bool(bool),
    /// Divert to another address, *cannot be printed*.
    Divert(Address),
    /// Decimal number.
    Float(f32),
    /// Integer number.
    Int(i32),
    /// Text string.
    String(String),
}

impl Variable {
    /// Return a string representation of the variable.
    pub fn to_string(&self, data: &FollowData) -> Result<String, InklingError> {
        match &self {
            Variable::Address(address) => match address {
                Address::Validated(AddressKind::Location { .. }) => {
                    let num_visited = get_num_visited(address, data)?;
                    Ok(format!("{}", num_visited))
                }
                Address::Validated(AddressKind::GlobalVariable { name }) => {
                    let variable = data.variables.get(name).unwrap();
                    variable.to_string(data)
                }
                _ => unimplemented!(),
            },
            Variable::Bool(value) => Ok(format!("{}", *value as u8)),
            Variable::Divert(..) => Err(InklingError::PrintInvalidVariable {
                name: String::new(),
                value: self.clone(),
            }),
            Variable::Float(value) => Ok(format!("{}", value)),
            Variable::Int(value) => Ok(format!("{}", value)),
            Variable::String(content) => Ok(content.clone()),
        }
    }
}

impl ValidateAddresses for Variable {
    fn validate(
        &mut self,
        current_address: &Address,
        data: &ValidateAddressData,
    ) -> Result<(), InvalidAddressError> {
        match self {
            Variable::Address(address) | Variable::Divert(address) => {
                address.validate(current_address, data)
            }
            Variable::Bool(..) | Variable::Float(..) | Variable::Int(..) | Variable::String(..) => {
                Ok(())
            }
        }
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        match self {
            Variable::Address(address) | Variable::Divert(address) => {
                address.all_addresses_are_valid()
            }
            Variable::Bool(..) | Variable::Float(..) | Variable::Int(..) | Variable::String(..) => {
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn booleans_are_printed_as_numbers() {
        let data = mock_follow_data(&[], &[]);

        assert_eq!(&Variable::Bool(true).to_string(&data).unwrap(), "1");
        assert_eq!(&Variable::Bool(false).to_string(&data).unwrap(), "0");
    }

    #[test]
    fn numbers_can_be_printed() {
        let data = mock_follow_data(&[], &[]);

        assert_eq!(&Variable::Int(5).to_string(&data).unwrap(), "5");
        assert_eq!(&Variable::Float(1.0).to_string(&data).unwrap(), "1");
        assert_eq!(&Variable::Float(1.35).to_string(&data).unwrap(), "1.35");
        assert_eq!(
            &Variable::Float(1.0000000003).to_string(&data).unwrap(),
            "1"
        );
    }

    #[test]
    fn strings_are_just_cloned() {
        let data = mock_follow_data(&[], &[]);

        assert_eq!(
            &Variable::String("two words".to_string())
                .to_string(&data)
                .unwrap(),
            "two words"
        );
    }

    #[test]
    fn addresses_are_printed_as_their_number_of_visits_if_they_are_locations() {
        let data = mock_follow_data(
            &[("tripoli", "cinema", 0), ("addis_ababa", "with_family", 3)],
            &[],
        );

        let tripoli = Address::from_parts_unchecked("tripoli", Some("cinema"));
        let addis_ababa = Address::from_parts_unchecked("addis_ababa", Some("with_family"));

        assert_eq!(&Variable::Address(tripoli).to_string(&data).unwrap(), "0");
        assert_eq!(
            &Variable::Address(addis_ababa).to_string(&data).unwrap(),
            "3"
        );
    }

    #[test]
    fn addresses_are_printed_as_the_contained_variables_if_they_are_variables() {
        let data = mock_follow_data(&[], &[("population", Variable::Int(1305))]);

        let address = Address::variable_unchecked("population");
        let variable = Variable::Address(address);

        assert_eq!(&variable.to_string(&data).unwrap(), "1305");
    }

    #[test]
    fn diverts_cannot_be_printed_but_yield_error() {
        let data = mock_follow_data(&[], &[]);
        let address = Address::from_parts_unchecked("tripoli", Some("cinema"));

        assert!(Variable::Divert(address).to_string(&data).is_err());
    }
}
