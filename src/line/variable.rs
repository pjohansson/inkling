//! Types of variables used in a story.

use crate::{
    error::{InklingError, InternalError, InvalidAddressError, VariableError, VariableErrorKind},
    follow::FollowData,
    knot::{get_num_visited, Address, AddressKind, ValidateAddressData, ValidateAddresses},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use std::cmp::Ordering;

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
///
/// # Type safety
/// As the enum consists of variants, the compiler cannot enforce static type checking
/// of `Variable`s. However, these variables *are* supposed to be static. Type safety
/// is enforced at runtime when values are updated.
///
/// # Examples
/// Any variant can be constructed (although `Address` and `Divert` are typically meant to
/// be created by the story processor, not the user):
/// ```
/// # use inkling::Variable;
/// let variable = Variable::Int(5);
/// let other_variable = Variable::String("I love you!".to_string());
/// ```
///
/// The `From` trait is implemented for integer, floating point, boolean and string types.
/// ```
/// # use inkling::Variable;
/// assert_eq!(Variable::from(5), Variable::Int(5));
/// assert_eq!(Variable::from(3.0), Variable::Float(3.0));
/// assert_eq!(Variable::from(true), Variable::Bool(true));
/// assert_eq!(Variable::from("💜"), Variable::String("💜".to_string()));
/// ```
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

    /// Return the value of a variable.
    ///
    /// If the variable is a number, boolean, string or divert a clone of the value is returned.
    ///
    /// If the variable is an address to another variable, we follow the address to that variable
    /// and return the value of that. This evaluates nested variables to the end.
    ///
    /// If the address is to a location in the story, the number of times that location has
    /// been visited is returned as an integer variable.
    pub(crate) fn as_value(&self, data: &FollowData) -> Result<Variable, InklingError> {
        match &self {
            Variable::Address(address) => match address {
                Address::Validated(AddressKind::Location { .. }) => {
                    let num_visited = get_num_visited(address, data)?;
                    Ok(Variable::Int(num_visited as i32))
                }
                Address::Validated(AddressKind::GlobalVariable { name }) => data
                    .variables
                    .get(name)
                    .ok_or(InklingError::InvalidVariable {
                        name: name.to_string(),
                    })
                    .and_then(|variable| variable.as_value(&data)),
                other => Err(InternalError::UseOfUnvalidatedAddress {
                    address: other.clone(),
                }
                .into()),
            },
            _ => Ok(self.clone()),
        }
    }

    /// Assign a new value to the variable.
    ///
    /// Variables are type static: assigning a new variable type (variant) is not allowed.
    /// This is checked before the assignment is made and an error will be raised.
    ///
    /// The given variable type is `Into<Variable>` which is implemented for all integer,
    /// floating point, boolean and string types.
    ///
    /// # Examples
    ///
    /// ## Assigning a new value
    /// ```
    /// # use inkling::Variable;
    /// let mut variable = Variable::Bool(true);
    ///
    /// variable.assign(Variable::Bool(false));
    /// assert_eq!(variable, Variable::Bool(false));
    /// ```
    ///
    /// ## Inferring input type
    /// ```
    /// # use inkling::Variable;
    /// let mut variable = Variable::Float(13.3);
    ///
    /// variable.assign(5.0);
    /// assert_eq!(variable, Variable::Float(5.0));
    /// ```
    ///
    /// ## Invalid other variable type assignment
    /// ```
    /// # use inkling::Variable;
    /// let mut variable = Variable::Int(10);
    ///
    /// assert!(variable.assign(Variable::Bool(true)).is_err());
    /// assert!(variable.assign(Variable::Float(5.0)).is_err());
    /// ```
    ///
    /// # Errors
    /// *   [`VariableTypeChange`][crate::error::InklingError::VariableTypeChange]:
    ///     if the variable types do not match.
    pub fn assign<T: Into<Variable>>(&mut self, value: T) -> Result<(), VariableError> {
        let inferred_value = value.into();

        match (&self, &inferred_value) {
            (Variable::Address(..), Variable::Address(..)) => (),
            (Variable::Bool(..), Variable::Bool(..)) => (),
            (Variable::Divert(..), Variable::Divert(..)) => (),
            (Variable::Float(..), Variable::Float(..)) => (),
            (Variable::Int(..), Variable::Int(..)) => (),
            (Variable::String(..), Variable::String(..)) => (),
            _ => {
                return Err(VariableError::from_kind(
                    self.clone(),
                    VariableErrorKind::NonMatchingAssignment {
                        other: inferred_value,
                    },
                ));
            }
        }

        *self = inferred_value;

        Ok(())
    }

    /// Assert whether a variable is equal to another.
    ///
    /// Different variable variants cannot be compared to each other, with one exception:
    /// integer and floating point numbers can be compared. If an integer is compared to
    /// a floating point number the integer will be cast to a float, then the comparison
    /// is made.
    ///
    /// # Examples
    /// ## Valid comparisons
    /// ```
    /// # use inkling::Variable;
    /// assert!(Variable::Int(5).equal_to(&Variable::Int(5)).unwrap());
    /// assert!(Variable::Int(5).equal_to(&Variable::Float(5.0)).unwrap());
    /// assert!(!Variable::Int(5).equal_to(&Variable::Float(5.1)).unwrap());
    /// assert!(!Variable::Bool(true).equal_to(&Variable::Bool(false)).unwrap());
    /// ```
    ///
    /// ## Invalid comparisons between types
    /// ```
    /// # use inkling::Variable;
    /// assert!(Variable::Int(1).equal_to(&Variable::Bool(true)).is_err());
    /// assert!(Variable::String("1".to_string()).equal_to(&Variable::Int(1)).is_err());
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidVariableComparison`][crate::error::InklingError::InvalidVariableComparison]:
    ///     if the two variables cannot be compared.
    pub fn equal_to(&self, other: &Variable) -> Result<bool, VariableError> {
        use Variable::*;

        match (&self, &other) {
            (Int(val1), Int(val2)) => Ok(val1.eq(val2)),
            (Int(val1), Float(val2)) => Ok((*val1 as f32).eq(val2)),
            (Float(val1), Int(val2)) => Ok(val1.eq(&(*val2 as f32))),
            (Float(val1), Float(val2)) => Ok(val1.eq(val2)),
            (Variable::String(val1), Variable::String(val2)) => Ok(val1.eq(val2)),
            (Bool(val1), Bool(val2)) => Ok(val1.eq(val2)),
            (Address(val1), Address(val2)) => Ok(val1.eq(val2)),
            (Divert(val1), Divert(val2)) => Ok(val1.eq(val2)),
            _ => Err(VariableError::from_kind(
                self.clone(),
                VariableErrorKind::InvalidComparison {
                    other: other.clone(),
                comparison: Ordering::Equal,
                },
            )),
        }
    }

    /// Assert whether a numeric variable value is greater than that of another.
    ///
    /// This operation is only valid for `Int` and `Float` variants. Those variants can
    /// be compared to each other. If an integer is compared to a floating point number
    /// the integer will be cast to a float, then the comparison is made.
    ///
    /// # Examples
    /// ## Valid comparisons between numbers
    /// ```
    /// # use inkling::Variable;
    /// assert!(Variable::Int(6).greater_than(&Variable::Int(5)).unwrap());
    /// assert!(!Variable::Int(4).greater_than(&Variable::Int(5)).unwrap());
    /// assert!(Variable::Int(5).greater_than(&Variable::Float(4.9)).unwrap());
    /// assert!(Variable::Float(5.1).greater_than(&Variable::Int(5)).unwrap());
    /// ```
    ///
    /// ## Invalid comparisons between non-numbers
    /// ```
    /// # use inkling::Variable;
    /// assert!(Variable::Int(1).greater_than(&Variable::Bool(false)).is_err());
    /// assert!(Variable::Bool(true).greater_than(&Variable::Bool(false)).is_err());
    /// assert!(Variable::from("hiya").greater_than(&Variable::from("hi")).is_err());
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidVariableComparison`][crate::error::InklingError::InvalidVariableComparison]:
    ///     if the two variables cannot be compared.
    pub fn greater_than(&self, other: &Variable) -> Result<bool, VariableError> {
        use Variable::*;

        match (&self, &other) {
            (Int(val1), Int(val2)) => Ok(val1.gt(val2)),
            (Int(val1), Float(val2)) => Ok((*val1 as f32).gt(val2)),
            (Float(val1), Int(val2)) => Ok(val1.gt(&(*val2 as f32))),
            (Float(val1), Float(val2)) => Ok(val1.gt(val2)),
            _ => Err(VariableError::from_kind(
                self.clone(),
                VariableErrorKind::InvalidComparison {
                    other: other.clone(),
                    comparison: Ordering::Greater,
                },
            )),
        }
    }

    /// Assert whether a numeric variable value is less than that of another.
    ///
    /// This operation is only valid for `Int` and `Float` variants. Those variants can
    /// be compared to each other. If an integer is compared to a floating point number
    /// the integer will be cast to a float, then the comparison is made.
    ///
    /// # Examples
    /// ## Valid comparisons between numbers
    /// ```
    /// # use inkling::Variable;
    /// assert!(Variable::Int(5).less_than(&Variable::Int(6)).unwrap());
    /// assert!(!Variable::Int(5).less_than(&Variable::Int(4)).unwrap());
    /// assert!(Variable::Int(5).less_than(&Variable::Float(5.1)).unwrap());
    /// assert!(Variable::Float(4.9).less_than(&Variable::Int(5)).unwrap());
    /// ```
    ///
    /// ## Invalid comparisons between non-numbers
    /// ```
    /// # use inkling::Variable;
    /// assert!(Variable::Int(0).less_than(&Variable::Bool(true)).is_err());
    /// assert!(Variable::Bool(false).less_than(&Variable::Bool(true)).is_err());
    /// assert!(Variable::from("hi").less_than(&Variable::from("hiya")).is_err());
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidVariableComparison`][crate::error::InklingError::InvalidVariableComparison]:
    ///     if the two variables cannot be compared.
    pub fn less_than(&self, other: &Variable) -> Result<bool, VariableError> {
        use Variable::*;

        match (&self, &other) {
            (Int(val1), Int(val2)) => Ok(val1.lt(val2)),
            (Int(val1), Float(val2)) => Ok((*val1 as f32).lt(val2)),
            (Float(val1), Int(val2)) => Ok(val1.lt(&(*val2 as f32))),
            (Float(val1), Float(val2)) => Ok(val1.lt(val2)),
            _ => Err(VariableError::from_kind(
                self.clone(),
                VariableErrorKind::InvalidComparison {
                    other: other.clone(),
                comparison: Ordering::Less,
                },
            )),
        }
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
    fn getting_value_from_all_non_address_variables_returns_the_variable() {
        let data = mock_follow_data(&[], &[]);

        assert_eq!(
            Variable::from(5).as_value(&data).unwrap(),
            Variable::from(5)
        );

        assert_eq!(
            Variable::from(5.0).as_value(&data).unwrap(),
            Variable::from(5.0)
        );

        assert_eq!(
            Variable::from("hiya").as_value(&data).unwrap(),
            Variable::from("hiya")
        );

        assert_eq!(
            Variable::from(true).as_value(&data).unwrap(),
            Variable::from(true)
        );

        let divert = Variable::Divert(Address::Raw("tripoli".to_string()));
        assert_eq!(divert.as_value(&data).unwrap(), divert);
    }

    #[test]
    fn getting_value_from_address_variable_of_location_gets_number_of_visits() {
        let data = mock_follow_data(
            &[("tripoli", "cinema", 0), ("addis_ababa", "with_family", 3)],
            &[],
        );

        let variable_one =
            Variable::Address(Address::from_parts_unchecked("tripoli", Some("cinema")));
        let variable_two = Variable::Address(Address::from_parts_unchecked(
            "addis_ababa",
            Some("with_family"),
        ));

        assert_eq!(variable_one.as_value(&data).unwrap(), Variable::Int(0));
        assert_eq!(variable_two.as_value(&data).unwrap(), Variable::Int(3));
    }

    #[test]
    fn getting_value_from_address_variable_of_global_variable_gets_value_of_that() {
        let data = mock_follow_data(&[], &[("population", Variable::Int(1305))]);

        let variable = Variable::Address(Address::variable_unchecked("population"));

        assert_eq!(variable.as_value(&data).unwrap(), Variable::Int(1305));
    }

    #[test]
    fn getting_value_from_invalid_global_variable_address_yields_error() {
        let data = mock_follow_data(&[], &[]);

        let variable = Variable::Address(Address::variable_unchecked("population"));

        match variable.as_value(&data) {
            Err(InklingError::InvalidVariable { .. }) => (),
            other => panic!(
                "expected `InklingError::InvalidVariable` but got {:?}",
                other
            ),
        }
    }

    #[test]
    fn getting_value_from_nested_address_variables_gets_to_the_bottom() {
        let data = mock_follow_data(
            &[("tripoli", "cinema", 5)],
            &[
                (
                    "nested1",
                    Variable::Address(Address::from_parts_unchecked("tripoli", Some("cinema"))),
                ),
                (
                    "nested2",
                    Variable::Address(Address::variable_unchecked("nested1")),
                ),
            ],
        );

        let variable_direct =
            Variable::Address(Address::from_parts_unchecked("tripoli", Some("cinema")));
        let variable_nested_one = Variable::Address(Address::variable_unchecked("nested1"));
        let variable_nested_two = Variable::Address(Address::variable_unchecked("nested2"));

        let result = variable_direct.as_value(&data).unwrap();
        assert_eq!(result, Variable::Int(5));

        assert_eq!(variable_nested_one.as_value(&data).unwrap(), result);
        assert_eq!(variable_nested_two.as_value(&data).unwrap(), result);
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

    #[test]
    fn numeric_variables_can_compare_to_each_other() {
        let int0 = Variable::Int(0);
        let int1 = Variable::Int(1);

        assert!(int1.equal_to(&int1).unwrap());
        assert!(!int1.equal_to(&int0).unwrap());

        assert!(int0.less_than(&int1).unwrap());
        assert!(!int0.less_than(&int0).unwrap());
        assert!(!int1.less_than(&int0).unwrap());

        assert!(int1.greater_than(&int0).unwrap());
        assert!(!int1.greater_than(&int1).unwrap());
        assert!(!int0.greater_than(&int1).unwrap());

        let float0 = Variable::Float(0.0);
        let float1 = Variable::Float(1.0);

        assert!(float1.equal_to(&float1).unwrap());
        assert!(!float1.equal_to(&float0).unwrap());

        assert!(float0.less_than(&float1).unwrap());
        assert!(!float0.less_than(&float0).unwrap());
        assert!(!float1.less_than(&float0).unwrap());

        assert!(float1.greater_than(&float0).unwrap());
        assert!(!float1.greater_than(&float1).unwrap());
        assert!(!float0.greater_than(&float1).unwrap());
    }

    #[test]
    fn integer_and_floating_point_values_can_compare_to_each_other() {
        assert!(Variable::Int(5).equal_to(&Variable::Float(5.0)).unwrap());
        assert!(Variable::Int(5).less_than(&Variable::Float(5.5)).unwrap());
        assert!(Variable::Float(5.5)
            .greater_than(&Variable::Int(5))
            .unwrap());
    }

    #[test]
    fn string_variables_can_do_equality_comparison_only() {
        let string1 = Variable::String("Hello, World!".to_string());
        let string2 = Variable::String("Hello!".to_string());

        assert!(string1.equal_to(&string1).unwrap());
        assert!(!string1.equal_to(&string2).unwrap());

        assert!(string1.less_than(&string2).is_err());
        assert!(string1.greater_than(&string2).is_err());
    }

    #[test]
    fn boolean_variables_can_do_equality_comparison_only() {
        let true_var = Variable::Bool(true);
        let false_var = Variable::Bool(false);

        assert!(true_var.equal_to(&true_var).unwrap());
        assert!(!true_var.equal_to(&false_var).unwrap());

        assert!(true_var.less_than(&false_var).is_err());
        assert!(true_var.greater_than(&false_var).is_err());
    }

    #[test]
    fn address_and_divert_variables_can_do_equality_against_their_own_variant() {
        let address1 = Variable::Address(Address::Raw("address1".to_string()));
        let address2 = Variable::Address(Address::Raw("address2".to_string()));

        assert!(address1.equal_to(&address1).unwrap());
        assert!(!address1.equal_to(&address2).unwrap());
        assert!(address1.less_than(&address2).is_err());
        assert!(address1.greater_than(&address2).is_err());

        let divert1 = Variable::Divert(Address::Raw("address1".to_string()));
        let divert2 = Variable::Divert(Address::Raw("address2".to_string()));

        assert!(divert1.equal_to(&divert1).unwrap());
        assert!(!divert1.equal_to(&divert2).unwrap());
        assert!(divert1.less_than(&divert2).is_err());
        assert!(divert1.greater_than(&divert2).is_err());
    }

    #[test]
    fn except_int_and_float_variants_cannot_compare_to_other() {
        let int = Variable::Int(5);
        let float = Variable::Float(6.0);
        let string = Variable::String("Hello".to_string());
        let boolean = Variable::Bool(true);
        let address = Variable::Address(Address::Raw("root".to_string()));
        let divert = Variable::Divert(Address::Raw("root".to_string()));

        assert!(int.equal_to(&string).is_err());
        assert!(int.equal_to(&boolean).is_err());
        assert!(int.equal_to(&address).is_err());
        assert!(int.equal_to(&divert).is_err());

        assert!(float.equal_to(&string).is_err());
        assert!(float.equal_to(&boolean).is_err());
        assert!(float.equal_to(&address).is_err());
        assert!(float.equal_to(&divert).is_err());

        assert!(string.equal_to(&int).is_err());
        assert!(string.equal_to(&float).is_err());
        assert!(string.equal_to(&boolean).is_err());
        assert!(string.equal_to(&address).is_err());
        assert!(string.equal_to(&divert).is_err());

        assert!(boolean.equal_to(&int).is_err());
        assert!(boolean.equal_to(&float).is_err());
        assert!(boolean.equal_to(&string).is_err());
        assert!(boolean.equal_to(&address).is_err());
        assert!(boolean.equal_to(&divert).is_err());

        assert!(address.equal_to(&int).is_err());
        assert!(address.equal_to(&float).is_err());
        assert!(address.equal_to(&string).is_err());
        assert!(address.equal_to(&boolean).is_err());
        assert!(address.equal_to(&divert).is_err());

        assert!(divert.equal_to(&int).is_err());
        assert!(divert.equal_to(&float).is_err());
        assert!(divert.equal_to(&string).is_err());
        assert!(divert.equal_to(&boolean).is_err());
        assert!(divert.equal_to(&address).is_err());
    }
}
