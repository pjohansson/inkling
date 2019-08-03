//! Types of variables used in a story.

use crate::{
    error::{InklingError, InternalError, InvalidAddressError},
    follow::FollowData,
    knot::{get_num_visited, Address, AddressKind, ValidateAddressData, ValidateAddresses},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Variable in a story.
///
/// Variables are typed and come in several variants, covering the basic needs of number
/// and string processing in `inkling`. When encountered in a story the processor will
/// parse a string from the value.
///
/// Be aware though, not all variants will evaluate to a string. Numbers and strings
/// make perfect sense to print: a divert to another location, not as much since it
/// has no meaning in text, just in the story internals. Take care not to use variables
/// that contain divert addresses in the text flow.
pub enum Variable {
    /// Address to a stitch or other variable.
    ///
    /// If the address is another variable in the story it will evaluate to that. If it
    /// is a location in the story it will evaluate to the number of times it has
    /// been visited.
    ///
    /// # Example
    /// If a line in the story contains the expression `{hazardous}` this will be treated
    /// as an address to either a knot/stitch or a global variable. The processor will
    /// take the value at the address and print that.
    Address(Address),
    /// True or false.
    ///
    /// When printed, the string representation of `true` is the number 1 and `false`
    /// is the number 0.
    Bool(bool),
    /// Divert to another address.
    ///
    /// This is fully internal and will never print to the story. If encountered as a variable
    /// in the text flow it will raise an error, since it should not be there.
    Divert(Address),
    /// Decimal number.
    ///
    /// Will print as that number, although floating point numbers can print weirdly sometimes.
    Float(f32),
    /// Integer number.
    ///
    /// Will print to that number.
    Int(i32),
    /// Text string.
    String(String),
}

impl Variable {
    /// Return a string representation of the variable.
    pub(crate) fn to_string(&self, data: &FollowData) -> Result<String, InklingError> {
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
                other => Err(InternalError::UseOfUnvalidatedAddress {
                    address: other.clone(),
                }
                .into()),
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

    /// Assign a new value to the variable.
    ///
    /// Variables are type static: assigning a new variable type (variant) is not allowed.
    /// This is checked before the assignment is made and an error will be raised.
    pub fn assign<T: Into<Variable>>(&mut self, value: T) -> Result<(), InklingError> {
        let inferred_value = value.into();

        match (&self, &inferred_value) {
            (Variable::Address(..), Variable::Address(..)) => (),
            (Variable::Bool(..), Variable::Bool(..)) => (),
            (Variable::Divert(..), Variable::Divert(..)) => (),
            (Variable::Float(..), Variable::Float(..)) => (),
            (Variable::Int(..), Variable::Int(..)) => (),
            (Variable::String(..), Variable::String(..)) => (),
            _ => {
                return Err(InklingError::VariableTypeChange {
                    from: self.clone(),
                    to: inferred_value.clone(),
                });
            }
        }

        *self = inferred_value;

        Ok(())
    }

    /// Get string representation of the variant.
    pub(crate) fn variant_string(&self) -> &str {
        match &self {
            Variable::Address(..) => "Address",
            Variable::Bool(..) => "Bool",
            Variable::Divert(..) => "DivertTarget",
            Variable::Float(..) => "Float",
            Variable::Int(..) => "Int",
            Variable::String(..) => "String",
        }
    }
}

macro_rules! impl_from {
    ( $variant:ident; $to:ty; $( $from:ty ),+ ) => {
        $(
            impl From<$from> for Variable {
                fn from(value: $from) -> Self {
                    Variable::$variant(value as $to)
                }
            }
        )*
    }
}

impl_from![Float; f32; f32, f64];
impl_from![Int; i32; u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize];

impl From<bool> for Variable {
    fn from(value: bool) -> Self {
        Variable::Bool(value)
    }
}

impl From<&str> for Variable {
    fn from(string: &str) -> Self {
        Variable::String(string.to_string())
    }
}

impl From<&String> for Variable {
    fn from(string: &String) -> Self {
        Variable::String(string.clone())
    }
}

impl From<String> for Variable {
    fn from(string: String) -> Self {
        Variable::String(string.clone())
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
    fn getting_string_representation_of_unvalidated_addresses_yields_error() {
        let data = mock_follow_data(&[], &[("population", Variable::Int(1305))]);

        let raw_address = Address::Raw("population".to_string());
        let variable = Variable::Address(raw_address.clone());

        match variable.to_string(&data) {
            Err(InklingError::Internal(InternalError::UseOfUnvalidatedAddress { address })) => {
                assert_eq!(address, raw_address);
            }
            other => panic!(
                "expected `InternalError::UseOfUnvalidatedAddress` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn diverts_cannot_be_printed_but_yield_error() {
        let data = mock_follow_data(&[], &[]);
        let address = Address::from_parts_unchecked("tripoli", Some("cinema"));

        assert!(Variable::Divert(address).to_string(&data).is_err());
    }

    #[test]
    fn assign_variable_value_updates_inner_value() {
        let mut variable = Variable::Int(5);
        variable.assign(Variable::Int(10)).unwrap();
        assert_eq!(variable, Variable::Int(10));
    }

    #[test]
    fn assign_variable_value_can_infer_type() {
        let mut variable = Variable::Int(5);

        variable.assign(10).unwrap();

        assert_eq!(variable, Variable::Int(10));
    }

    #[test]
    fn assign_variable_value_cannot_change_variable_type() {
        let mut variable = Variable::Int(5);

        assert!(variable.assign(Variable::Bool(true)).is_err());
        assert!(variable.assign(Variable::Float(5.0)).is_err());
        assert!(variable
            .assign(Variable::String("help".to_string()))
            .is_err());
    }
}
