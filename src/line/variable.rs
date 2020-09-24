//! Types of variables used in a story.

use crate::{
    error::{
        parse::validate::ValidationError,
        utils::MetaData,
        variable::{VariableError, VariableErrorKind},
        InklingError, InternalError,
    },
    follow::FollowData,
    knot::{get_num_visited, Address, AddressKind},
    story::{
        validate::{ValidateContent, ValidationData},
        Location,
    },
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
    /// True or false.
    ///
    /// When printed, the string representation of `true` is the number 1 and `false`
    /// is the number 0.
    Bool(bool),
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
    /// Divert to another address.
    ///
    /// This is fully internal and will never print to the story. If encountered as a variable
    /// in the text flow it will raise an error, since it should not be there.
    Divert(Address),
    /// Address to a stitch or other variable.
    ///
    /// If the address is another variable in the story it will evaluate to that. If it
    /// is a location in the story it will evaluate to the number of times it has
    /// been visited.
    ///
    /// # Note
    /// This kind is fully internal in the story script and cannot be used for global
    /// variables.
    ///
    /// # Example
    /// If a line in the story contains the expression `{hazardous}` this will be treated
    /// as an address to either a knot/stitch or a global variable. The processor will
    /// take the value at the address and print that.
    Address(Address),
}

impl Variable {
    /// Return the value of the variable as a string, if it is printable.
    ///
    /// `Int`, `Float`, `String` and `Bool` are printable variants because they can exist
    /// outside of the context of `inkling`.
    ///
    /// `Divert` and `Address` are internal `inkling` constructs and have no meaning
    /// to the player. Variables of such kinds yield `None`. They can be printed
    /// using the `to_string_unchecked` method.
    ///
    /// # Examples
    ///
    /// ## Printable values
    ///
    /// ```
    /// # use inkling::Variable;
    /// assert_eq!(Variable::Int(3).to_string(), Some("3".to_string()));
    /// assert_eq!(Variable::Float(0.5).to_string(), Some("0.5".to_string()));
    /// assert_eq!(Variable::Bool(true).to_string(), Some("true".to_string()));
    /// assert_eq!(Variable::Bool(false).to_string(), Some("false".to_string()));
    /// assert_eq!(Variable::String("String".into()).to_string(), Some("String".to_string()));
    /// ```
    ///
    /// ## Unprintable divert
    ///
    /// ```
    /// # use inkling::read_story_from_string;
    /// let content = "\
    /// VAR destination = -> château
    ///
    /// === château ===
    /// Meg arrives at the mansion.
    /// ";
    ///
    /// let story = read_story_from_string(content).unwrap();
    /// let variable = story.get_variable("destination").unwrap();
    ///
    /// assert_eq!(variable.to_string(), None);
    /// ```
    pub fn to_string(&self) -> Option<String> {
        match &self {
            Variable::Bool(value) => Some(format!("{}", value)),
            Variable::Float(value) => Some(format!("{}", value)),
            Variable::Int(value) => Some(format!("{}", value)),
            Variable::String(string) => Some(format!("{}", string)),
            Variable::Divert(_) | Variable::Address(_) => None,
        }
    }

    /// Return the value of the variable as a string without checking that it is printable.
    ///
    /// `Int`, `Float`, `String` and `Bool` are printable variants because they can exist
    /// outside of the context of `inkling`.
    ///
    /// `Divert` and `Address` are internal `inkling` constructs and have no meaning
    /// to the player. This function forces them to print their internal addresses.
    /// Note that global variables can never be of kind `Address`, thus they should
    /// never be exposed to the user. `Divert` variables, however, may be.
    ///
    /// # Examples
    ///
    /// ## Printable values
    ///
    /// ```
    /// # use inkling::Variable;
    /// assert_eq!(&Variable::Int(3).to_string_unchecked(), "3");
    /// assert_eq!(&Variable::Float(0.5).to_string_unchecked(), "0.5");
    /// assert_eq!(&Variable::Bool(true).to_string_unchecked(), "true");
    /// assert_eq!(&Variable::Bool(false).to_string_unchecked(), "false");
    /// assert_eq!(&Variable::String("String".into()).to_string_unchecked(), "String");
    /// ```
    ///
    /// ## Unprintable divert
    ///
    /// ```
    /// # use inkling::read_story_from_string;
    /// let content = "\
    /// VAR destination = -> château
    ///
    /// === château ===
    /// Meg arrives at the mansion.
    /// ";
    ///
    /// let story = read_story_from_string(content).unwrap();
    /// let variable = story.get_variable("destination").unwrap();
    ///
    /// assert_eq!(&variable.to_string_unchecked(), "-> château");
    /// ```
    pub fn to_string_unchecked(&self) -> String {
        match &self {
            Variable::Divert(address) => format!("-> {}", address.to_string()),
            // `Address` variants are fully internal and should not be possible to be operated
            // on by a caller. As a fallback we return the address as a string.
            Variable::Address(address) => address.to_string(),
            _ => self.to_string().unwrap(),
        }
    }

    /// Get the target `Location` of a `Variable::Divert` variant.
    ///
    /// `Variables` which are not of `Divert` type yield `None`.
    ///
    /// # Examples
    /// ```
    /// # use inkling::{read_story_from_string, Location, Variable};
    /// let content = "\
    /// VAR location = -> mirandas_den.dream
    /// ";
    ///
    /// let story = read_story_from_string(content).unwrap();
    /// let variable = story.get_variable("location").unwrap();
    /// assert_eq!(
    ///     variable.get_location(),
    ///     Some(Location {
    ///         knot: "mirandas_den".to_string(),
    ///         stitch: Some("dream".to_string()),
    ///     })
    /// );
    ///
    /// assert!(Variable::Int(5).get_location().is_none());
    /// assert!(Variable::Float(3.0).get_location().is_none());
    /// assert!(Variable::Bool(true).get_location().is_none());
    /// assert!(Variable::String("knot.stitch".to_string()).get_location().is_none());
    pub fn get_location(&self) -> Option<Location> {
        match self {
            Variable::Divert(address) => Some(Location::from(address.to_string().as_ref())),
            _ => None,
        }
    }

    /// Return a string representation of the variable for printing in the story text.
    ///
    /// If the variable is an address, the address will be followed until a non-address
    /// variable is found. That variable's string representation will then be returned.
    pub(crate) fn to_string_internal(&self, data: &FollowData) -> Result<String, InklingError> {
        match &self {
            Variable::Address(address) => match address {
                Address::Validated(AddressKind::Location { .. }) => {
                    let num_visited = get_num_visited(address, data)?;
                    Ok(format!("{}", num_visited))
                }
                Address::Validated(AddressKind::GlobalVariable { name }) => data
                    .variables
                    .get(name)
                    .ok_or(InklingError::InvalidVariable {
                        name: name.to_string(),
                    })
                    .and_then(|variable_info| variable_info.variable.to_string_internal(data)),
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
    /// Return a simple string representation of the variable which does not follow addresses.
    ///
    /// This corresponds to a string which the variable could be parsed from.
    ///
    /// Used for printing errors.
    pub(crate) fn to_error_string(&self) -> String {
        match &self {
            Variable::Address(address) => address.to_string(),
            Variable::Bool(value) => format!("{}", value),
            Variable::Float(value) => format!("{}", value),
            Variable::Int(value) => format!("{}", value),
            Variable::String(string) => format!("\"{}\"", string),
            Variable::Divert(address) => format!("-> {}", address.to_string()),
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
                    .and_then(|info| info.variable.as_value(&data)),
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
    /// *   [`NonMatchingAssignment`][crate::error::variable::VariableErrorKind::NonMatchingAssignment]:
    ///     if the variable types do not match.
    pub fn assign<T: Into<Variable>>(&mut self, value: T) -> Result<(), VariableError> {
        use Variable::*;

        let inferred_value = value.into();

        match (&self, &inferred_value) {
            (Address(..), Address(..)) => (),
            (Bool(..), Bool(..)) => (),
            (Divert(..), Divert(..)) => (),
            (Float(..), Float(..)) => (),
            (Int(..), Int(..)) => (),
            (String(..), String(..)) => (),
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

    /// Add the value of a variable to that of another.
    ///
    /// This operation is valid for integer, floating point and string variables.
    /// Integer and floating point variables simply adds the numbers together. String
    /// variables concatenate their strings.
    ///
    /// Integer and floating point values can be added to one another. If so, the integer
    /// is cast into a floating point number before the operation and the variable is returned
    /// as a floating point type.
    ///
    /// # Examples
    /// ## Numeric addition
    /// ```
    /// # use inkling::Variable;
    /// assert_eq!(
    ///     Variable::Int(1).add(&Variable::Int(2)).unwrap(),
    ///     Variable::Int(3)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Float(1.0).add(&Variable::Float(2.0)).unwrap(),
    ///     Variable::Float(3.0)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Int(1).add(&Variable::Float(2.0)).unwrap(),
    ///     Variable::Float(3.0)
    /// );
    /// ```
    ///
    /// ## String concatenation
    /// ```
    /// # use inkling::Variable;
    /// let string1 = Variable::from("hi");
    /// let string2 = Variable::from("ya!");
    ///
    /// assert_eq!(
    ///     string1.add(&string2).unwrap(),
    ///     Variable::String("hiya!".to_string())
    /// );
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidOperation`][crate::error::variable::VariableErrorKind::InvalidOperation]:
    ///     if the variables cannot perform this operation.
    pub fn add(&self, other: &Variable) -> Result<Variable, VariableError> {
        use Variable::*;

        match (&self, other) {
            (Int(val1), Int(val2)) => Ok(Int(val1 + val2)),
            (Int(val1), Float(val2)) => Ok(Float(*val1 as f32 + val2)),
            (Float(val1), Int(val2)) => Ok(Float(val1 + *val2 as f32)),
            (Float(val1), Float(val2)) => Ok(Float(val1 + val2)),
            (String(s1), String(s2)) => Ok(String(format!("{}{}", s1, s2))),
            _ => Err(VariableError::from_kind(
                self.clone(),
                VariableErrorKind::InvalidOperation {
                    other: other.clone(),
                    operator: '+',
                },
            )),
        }
    }

    /// Subtract the value of a variable from that of another.
    ///
    /// This operation is valid for integer and floating point variables.
    ///
    /// Integer and floating point values can be subtracted from one another. If so, the integer
    /// is cast into a floating point number before the operation and the variable is returned
    /// as a floating point type.
    ///
    /// # Examples
    /// ```
    /// # use inkling::Variable;
    /// assert_eq!(
    ///     Variable::Int(1).subtract(&Variable::Int(2)).unwrap(),
    ///     Variable::Int(-1)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Float(1.0).subtract(&Variable::Float(2.0)).unwrap(),
    ///     Variable::Float(-1.0)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Int(1).subtract(&Variable::Float(2.0)).unwrap(),
    ///     Variable::Float(-1.0)
    /// );
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidOperation`][crate::error::variable::VariableErrorKind::InvalidOperation]:
    ///     if the variables cannot perform this operation.
    pub fn subtract(&self, other: &Variable) -> Result<Variable, VariableError> {
        use Variable::*;

        match (&self, other) {
            (Int(val1), Int(val2)) => Ok(Int(val1 - val2)),
            (Int(val1), Float(val2)) => Ok(Float(*val1 as f32 - val2)),
            (Float(val1), Int(val2)) => Ok(Float(val1 - *val2 as f32)),
            (Float(val1), Float(val2)) => Ok(Float(val1 - val2)),
            _ => Err(VariableError::from_kind(
                self.clone(),
                VariableErrorKind::InvalidOperation {
                    other: other.clone(),
                    operator: '-',
                },
            )),
        }
    }

    /// Multiply the value of a variable with that of another.
    ///
    /// This operation is valid for integer and floating point variables.
    ///
    /// Integer and floating point values can be multiplied with one another. If so, the integer
    /// is cast into a floating point number before the operation and the variable is returned
    /// as a floating point type.
    ///
    /// # Examples
    /// ```
    /// # use inkling::Variable;
    /// assert_eq!(
    ///     Variable::Int(2).multiply(&Variable::Int(3)).unwrap(),
    ///     Variable::Int(6)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Float(2.0).multiply(&Variable::Float(3.0)).unwrap(),
    ///     Variable::Float(6.0)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Int(2).multiply(&Variable::Float(3.0)).unwrap(),
    ///     Variable::Float(6.0)
    /// );
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidOperation`][crate::error::variable::VariableErrorKind::InvalidOperation]:
    ///     if the variables cannot perform this operation.
    pub fn multiply(&self, other: &Variable) -> Result<Variable, VariableError> {
        use Variable::*;

        match (&self, other) {
            (Int(val1), Int(val2)) => Ok(Int(val1 * val2)),
            (Int(val1), Float(val2)) => Ok(Float(*val1 as f32 * val2)),
            (Float(val1), Int(val2)) => Ok(Float(val1 * *val2 as f32)),
            (Float(val1), Float(val2)) => Ok(Float(val1 * val2)),
            _ => Err(VariableError::from_kind(
                self.clone(),
                VariableErrorKind::InvalidOperation {
                    other: other.clone(),
                    operator: '*',
                },
            )),
        }
    }

    /// Divide the value of a variable with that of another.
    ///
    /// This operation is valid for integer and floating point variables.
    ///
    /// Integer and floating point values can be divided with one another. If so, the integer
    /// is cast into a floating point number before the operation and the variable is returned
    /// as a floating point type.
    ///
    /// # Examples
    /// ```
    /// # use inkling::Variable;
    /// assert_eq!(
    ///     Variable::Int(5).divide(&Variable::Int(2)).unwrap(),
    ///     Variable::Int(2)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Float(5.0).divide(&Variable::Float(2.0)).unwrap(),
    ///     Variable::Float(2.5)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Int(5).divide(&Variable::Float(2.0)).unwrap(),
    ///     Variable::Float(2.5)
    /// );
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidOperation`][crate::error::variable::VariableErrorKind::InvalidOperation]:
    ///     if the variables cannot perform this operation.
    /// *   [`DividedByZero`][crate::error::variable::VariableErrorKind::DividedByZero]:
    ///     if the `other` variable value was 0.
    pub fn divide(&self, other: &Variable) -> Result<Variable, VariableError> {
        use Variable::*;

        match (&self, other) {
            (_, Int(0)) => Err(VariableErrorKind::DividedByZero {
                other: other.clone(),
                operator: '/',
            }),
            (_, Float(v)) if *v == 0.0 => Err(VariableErrorKind::DividedByZero {
                other: other.clone(),
                operator: '/',
            }),
            (Int(val1), Int(val2)) => Ok(Int(val1 / val2)),
            (Int(val1), Float(val2)) => Ok(Float(*val1 as f32 / val2)),
            (Float(val1), Int(val2)) => Ok(Float(val1 / *val2 as f32)),
            (Float(val1), Float(val2)) => Ok(Float(val1 / val2)),
            _ => Err(VariableErrorKind::InvalidOperation {
                other: other.clone(),
                operator: '/',
            }),
        }
        .map_err(|kind| VariableError::from_kind(self.clone(), kind))
    }

    /// Find the remainder after dividing the value of a variable with that of another.
    ///
    /// This operation is valid for integer and floating point variables.
    ///
    /// Integer and floating point values can perform this operation with one another. If so,
    /// the integer is cast into a floating point number before the operation and the variable
    /// is returned as a floating point type.
    ///
    /// # Examples
    /// ```
    /// # use inkling::Variable;
    /// assert_eq!(
    ///     Variable::Int(5).remainder(&Variable::Int(2)).unwrap(),
    ///     Variable::Int(1)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Float(5.0).remainder(&Variable::Float(2.0)).unwrap(),
    ///     Variable::Float(1.0)
    /// );
    ///
    /// assert_eq!(
    ///     Variable::Int(5).remainder(&Variable::Float(2.0)).unwrap(),
    ///     Variable::Float(1.0)
    /// );
    /// ```
    ///
    /// # Errors
    /// *   [`InvalidOperation`][crate::error::variable::VariableErrorKind::InvalidOperation]:
    ///     if the variables cannot perform this operation.
    /// *   [`DividedByZero`][crate::error::variable::VariableErrorKind::DividedByZero]:
    ///     if the `other` variable value was 0.
    pub fn remainder(&self, other: &Variable) -> Result<Variable, VariableError> {
        use Variable::*;

        match (&self, other) {
            (_, Int(0)) => Err(VariableErrorKind::DividedByZero {
                other: other.clone(),
                operator: '%',
            }),
            (_, Float(v)) if *v == 0.0 => Err(VariableErrorKind::DividedByZero {
                other: other.clone(),
                operator: '%',
            }),
            (Int(val1), Int(val2)) => Ok(Int(val1 % val2)),
            (Int(val1), Float(val2)) => Ok(Float(*val1 as f32 % val2)),
            (Float(val1), Int(val2)) => Ok(Float(val1 % *val2 as f32)),
            (Float(val1), Float(val2)) => Ok(Float(val1 % val2)),
            _ => Err(VariableErrorKind::InvalidOperation {
                other: other.clone(),
                operator: '%',
            }),
        }
        .map_err(|kind| VariableError::from_kind(self.clone(), kind))
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
    /// *   [`InvalidComparison`][crate::error::variable::VariableErrorKind::InvalidComparison]:
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
    /// *   [`InvalidComparison`][crate::error::variable::VariableErrorKind::InvalidComparison]:
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
    /// *   [`InvalidComparison`][crate::error::variable::VariableErrorKind::InvalidComparison]:
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

impl ValidateContent for Variable {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        current_location: &Address,
        meta_data: &MetaData,
        data: &ValidationData,
    ) {
        match self {
            Variable::Address(address) | Variable::Divert(address) => {
                address.validate(error, current_location, meta_data, data);
            }
            Variable::Bool(..) | Variable::Float(..) | Variable::Int(..) | Variable::String(..) => {
                ()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{follow::FollowDataBuilder, story::types::VariableInfo};

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
            .enumerate()
            .map(|(i, (name, var))| (name, VariableInfo::new(var, i)))
            .map(|(name, var)| (name.to_string(), var))
            .collect();

        FollowDataBuilder::new()
            .with_knots(knot_visit_counts)
            .with_variables(variables)
            .build()
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
    fn variable_to_string_returns_numbers_for_int_and_float_variables() {
        assert_eq!(&Variable::Int(5).to_string().unwrap(), "5");
        assert_eq!(Variable::Float(3.4).to_string(), Some(format!("{}", 3.4)));
    }

    #[test]
    fn variable_to_string_returns_bool_variables_as_text() {
        assert_eq!(&Variable::Bool(false).to_string().unwrap(), "false");
        assert_eq!(&Variable::Bool(true).to_string().unwrap(), "true");
    }

    #[test]
    fn variable_to_string_returns_string_variables_as_the_string() {
        let s = "A String".to_string();

        assert_eq!(&Variable::String(s.clone()).to_string().unwrap(), &s);
    }

    #[test]
    fn variable_to_string_returns_none_for_address_and_divert_variables() {
        let knot = Address::from_parts_unchecked("tripoli", None);
        let stitch = Address::from_parts_unchecked("tripoli", Some("cinema"));

        assert!(Variable::Divert(knot.clone()).to_string().is_none());
        assert!(Variable::Divert(stitch.clone()).to_string().is_none());

        assert!(Variable::Address(knot.clone()).to_string().is_none());
        assert!(Variable::Address(stitch.clone()).to_string().is_none());
    }

    #[test]
    fn to_string_unchecked_returns_same_result_as_to_string_for_printable_kinds() {
        let var_int = Variable::Int(5);
        let var_float = Variable::Float(3.0);
        let var_true = Variable::Bool(true);
        let var_false = Variable::Bool(false);
        let var_string = Variable::String("A String".to_string());

        assert_eq!(var_int.to_string().unwrap(), var_int.to_string_unchecked());
        assert_eq!(
            var_float.to_string().unwrap(),
            var_float.to_string_unchecked()
        );
        assert_eq!(
            var_string.to_string().unwrap(),
            var_string.to_string_unchecked()
        );
        assert_eq!(
            var_true.to_string().unwrap(),
            var_true.to_string_unchecked()
        );
        assert_eq!(
            var_false.to_string().unwrap(),
            var_false.to_string_unchecked()
        );
    }

    #[test]
    fn to_string_unchecked_returns_diverts_with_preceeding_divert_marker() {
        let knot = Address::from_parts_unchecked("tripoli", None);
        let stitch = Address::from_parts_unchecked("tripoli", Some("cinema"));

        assert_eq!(&Variable::Divert(knot).to_string_unchecked(), "-> tripoli");
        assert_eq!(
            &Variable::Divert(stitch).to_string_unchecked(),
            "-> tripoli.cinema"
        );
    }

    #[test]
    fn to_string_unchecked_returns_addresses_as_text() {
        let knot = Address::from_parts_unchecked("tripoli", None);
        let stitch = Address::from_parts_unchecked("tripoli", Some("cinema"));

        assert_eq!(&Variable::Address(knot).to_string_unchecked(), "tripoli");
        assert_eq!(
            &Variable::Address(stitch).to_string_unchecked(),
            "tripoli.cinema"
        );
    }

    #[test]
    fn booleans_are_internally_printed_as_numbers() {
        let data = mock_follow_data(&[], &[]);

        assert_eq!(
            &Variable::Bool(true).to_string_internal(&data).unwrap(),
            "1"
        );
        assert_eq!(
            &Variable::Bool(false).to_string_internal(&data).unwrap(),
            "0"
        );
    }

    #[test]
    fn numbers_can_be_internally_printed() {
        let data = mock_follow_data(&[], &[]);

        assert_eq!(&Variable::Int(5).to_string_internal(&data).unwrap(), "5");
        assert_eq!(
            &Variable::Float(1.0).to_string_internal(&data).unwrap(),
            "1"
        );
        assert_eq!(
            &Variable::Float(1.35).to_string_internal(&data).unwrap(),
            "1.35"
        );
        assert_eq!(
            &Variable::Float(1.0000000003)
                .to_string_internal(&data)
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn strings_are_just_cloned_when_internally_printed() {
        let data = mock_follow_data(&[], &[]);

        assert_eq!(
            &Variable::String("two words".to_string())
                .to_string_internal(&data)
                .unwrap(),
            "two words"
        );
    }

    #[test]
    fn addresses_are_internally_printed_as_their_number_of_visits_if_they_are_locations() {
        let data = mock_follow_data(
            &[("tripoli", "cinema", 0), ("addis_ababa", "with_family", 3)],
            &[],
        );

        let tripoli = Address::from_parts_unchecked("tripoli", Some("cinema"));
        let addis_ababa = Address::from_parts_unchecked("addis_ababa", Some("with_family"));

        assert_eq!(
            &Variable::Address(tripoli)
                .to_string_internal(&data)
                .unwrap(),
            "0"
        );
        assert_eq!(
            &Variable::Address(addis_ababa)
                .to_string_internal(&data)
                .unwrap(),
            "3"
        );
    }

    #[test]
    fn addresses_are_internally_printed_as_the_contained_variables_if_they_are_variables() {
        let data = mock_follow_data(&[], &[("population", Variable::Int(1305))]);

        let address = Address::variable_unchecked("population");
        let variable = Variable::Address(address);

        assert_eq!(&variable.to_string_internal(&data).unwrap(), "1305");
    }

    #[test]
    fn getting_internal_string_representation_of_unvalidated_addresses_yields_error() {
        let data = mock_follow_data(&[], &[("population", Variable::Int(1305))]);

        let raw_address = Address::Raw("population".to_string());
        let variable = Variable::Address(raw_address.clone());

        match variable.to_string_internal(&data) {
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
    fn diverts_cannot_be_internally_printed_but_yield_error() {
        let data = mock_follow_data(&[], &[]);
        let address = Address::from_parts_unchecked("tripoli", Some("cinema"));

        assert!(Variable::Divert(address).to_string_internal(&data).is_err());
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

    #[test]
    fn dividing_by_infinity_yields_error() {
        assert!(Variable::from(1).divide(&0.into()).is_err());
        assert!(Variable::from(1.0).divide(&0.0.into()).is_err());

        assert!(Variable::from(1).remainder(&0.into()).is_err());
        assert!(Variable::from(1.0).remainder(&0.0.into()).is_err());
    }

    #[test]
    fn get_location_yields_target_for_divert() {
        let address = Address::from_parts_unchecked("tripoli", Some("cinema"));
        let location = Location::with_stitch("tripoli", "cinema");

        assert_eq!(Variable::Divert(address).get_location(), Some(location));
    }

    #[test]
    fn get_location_yields_none_for_anything_but_diverts() {
        let address = Address::from_parts_unchecked("tripoli", Some("cinema"));

        assert!(Variable::Int(5).get_location().is_none());
        assert!(Variable::Float(3.0).get_location().is_none());
        assert!(Variable::Bool(true).get_location().is_none());
        assert!(Variable::String("knot.stitch".to_string())
            .get_location()
            .is_none());
        assert!(Variable::Address(address).get_location().is_none());
    }
}
