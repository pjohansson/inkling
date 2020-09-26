//! Validated addresses to content in a story.

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use crate::{
    consts::{DONE_KNOT, END_KNOT, ROOT_KNOT_NAME},
    error::{
        parse::{
            address::{InvalidAddressError, InvalidAddressErrorKind},
            validate::ValidationError,
        },
        utils::MetaData,
        InternalError,
    },
    knot::KnotSet,
    story::{
        validate::{KnotValidationInfo, ValidateContent, ValidationData},
        Logger,
    },
};

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// A verified address to a `Knot` or `Stitch` in the story.
///
/// Used to leverage the type system and ensure that functions which require complete addresses
/// get them.
pub enum Address {
    /// An address that has been validated and is guarantueed to resolve.
    Validated(AddressKind),
    /// This string-formatted address has not yet been validated.
    Raw(String),
    /// Divert address to mark that a story is finished.
    End,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
pub enum AddressKind {
    Location { knot: String, stitch: String },
    GlobalVariable { name: String },
}

impl From<AddressKind> for Address {
    fn from(address: AddressKind) -> Self {
        Address::Validated(address)
    }
}

use crate::story::Location;

impl Address {
    /// Return an address from a string that is just a knot name.
    ///
    /// The knot name is verified as present in the `KnotSet` set. The `Stitch` is set
    /// as the default for the found `Knot`.
    pub fn from_root_knot(
        root_knot_name: &str,
        knots: &KnotSet,
    ) -> Result<Self, InvalidAddressErrorKind> {
        let knot = knots
            .get(root_knot_name)
            .ok_or(InvalidAddressErrorKind::UnknownKnot {
                knot_name: root_knot_name.to_string(),
            })?;

        Ok(Address::Validated(AddressKind::Location {
            knot: root_knot_name.to_string(),
            stitch: knot.default_stitch.clone(),
        }))
    }

    /// Validate that a specified location exists in the knotset and create it's `Address`.
    pub fn from_location(
        location: &Location,
        knots: &KnotSet,
    ) -> Result<Self, InvalidAddressErrorKind> {
        let knot = knots
            .get(&location.knot)
            .ok_or(InvalidAddressErrorKind::UnknownKnot {
                knot_name: location.knot.to_string(),
            })?;

        let stitch_name = location.stitch.as_ref().unwrap_or(&knot.default_stitch);

        if knot.stitches.contains_key(stitch_name) {
            Ok(Address::Validated(AddressKind::Location {
                knot: location.knot.clone(),
                stitch: stitch_name.clone(),
            }))
        } else {
            Err(InvalidAddressErrorKind::UnknownStitch {
                knot_name: location.knot.clone(),
                stitch_name: stitch_name.clone(),
            })
        }
    }

    /// Get the knot name of a validated address.
    pub fn get_knot(&self) -> Result<&str, InternalError> {
        match self {
            Address::Validated(AddressKind::Location { knot, .. }) => Ok(knot),
            Address::Validated(AddressKind::GlobalVariable { name }) => {
                Err(InternalError::UseOfVariableAsLocation { name: name.clone() })
            }
            _ => Err(InternalError::UseOfUnvalidatedAddress {
                address: self.clone(),
            }),
        }
    }

    /// Get the stitch name of a validateed address.
    pub fn get_stitch(&self) -> Result<&str, InternalError> {
        match self {
            Address::Validated(AddressKind::Location { stitch, .. }) => Ok(stitch),
            Address::Validated(AddressKind::GlobalVariable { name }) => {
                Err(InternalError::UseOfVariableAsLocation { name: name.clone() })
            }
            _ => Err(InternalError::UseOfUnvalidatedAddress {
                address: self.clone(),
            }),
        }
    }

    /// Get knot and stitch names from a validated address.
    pub fn get_knot_and_stitch(&self) -> Result<(&str, &str), InternalError> {
        match self {
            Address::Validated(AddressKind::Location { knot, stitch }) => Ok((knot, stitch)),
            Address::Validated(AddressKind::GlobalVariable { name }) => {
                Err(InternalError::UseOfVariableAsLocation { name: name.clone() })
            }
            _ => Err(InternalError::UseOfUnvalidatedAddress {
                address: self.clone(),
            }),
        }
    }

    /// Get a string representation of the address as `Ink` would write it.
    pub fn to_string(&self) -> String {
        match &self {
            Address::Validated(AddressKind::GlobalVariable { name }) => name.clone(),
            Address::Validated(AddressKind::Location { knot, stitch }) => {
                if stitch.as_str() == ROOT_KNOT_NAME {
                    format!("{}", knot)
                } else {
                    format!("{}.{}", knot, stitch)
                }
            }
            Address::Raw(content) => content.clone(),
            Address::End => "END".to_string(),
        }
    }

    /// Validate the `Address` if it is `Raw`.
    fn validate_internal(
        &mut self,
        current_location: &Address,
        data: &ValidationData,
    ) -> Result<(), InvalidAddressErrorKind> {
        match self {
            Address::Raw(ref target) if target == DONE_KNOT || target == END_KNOT => {
                *self = Address::End;
            }
            Address::Raw(ref target) => {
                let address = match split_address_into_parts(target.trim())? {
                    (knot, Some(stitch)) => get_location_from_parts(knot, stitch, &data.knots)?,
                    (needle, None) => get_address_from_needle(needle, current_location, data)?,
                }
                .into();

                *self = address;
            }
            Address::Validated { .. } | Address::End => (),
        }

        Ok(())
    }
}

impl ValidateContent for Address {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        _log: &mut Logger,
        current_location: &Address,
        meta_data: &MetaData,
        data: &ValidationData,
    ) {
        if let Err(kind) = self.validate_internal(current_location, data) {
            let err = InvalidAddressError {
                kind,
                meta_data: meta_data.clone(),
            };

            if error
                .invalid_address_errors
                .last()
                .map(|last_err| last_err != &err)
                .unwrap_or(true)
            {
                error.invalid_address_errors.push(err);
            }
        }
    }
}

/// Split an address into constituent parts if possible.
///
/// The split is done at a dot ('.') marker. If one exists, split at it and return the parts.
/// Otherwise return the entire string.
fn split_address_into_parts(
    address: &str,
) -> Result<(String, Option<String>), InvalidAddressErrorKind> {
    if let Some(i) = address.find('.') {
        let knot = address.get(..i).unwrap();

        let stitch = address
            .get(i + 1..)
            .ok_or(InvalidAddressErrorKind::BadFormat {
                line: address.to_string(),
            })?;

        Ok((knot.to_string(), Some(stitch.to_string())))
    } else {
        Ok((address.to_string(), None))
    }
}

/// Verify and return the full address to a stitch from its parts.
fn get_location_from_parts(
    knot_name: String,
    stitch_name: String,
    knots: &HashMap<String, KnotValidationInfo>,
) -> Result<AddressKind, InvalidAddressErrorKind> {
    let KnotValidationInfo { stitches, .. } =
        knots
            .get(&knot_name)
            .ok_or(InvalidAddressErrorKind::UnknownKnot {
                knot_name: knot_name.clone(),
            })?;

    if stitches.contains_key(&stitch_name) {
        Ok(AddressKind::Location {
            knot: knot_name,
            stitch: stitch_name,
        })
    } else {
        Err(InvalidAddressErrorKind::UnknownStitch {
            knot_name: knot_name.clone(),
            stitch_name: stitch_name.clone(),
        })
    }
}

/// Return a validated address from a single name.
///
/// Internal addresses are relative to the current knot. If one is found in the current knot,
/// the knot name and the address is returned. Otherwise the default stitch from a knot
/// with the name is returned.
///
/// If the name is not found in the current knot's stitches, or in the set of knot names,
/// the variable listing is searched. If a match is found the address will be returned
/// as a global variable.
fn get_address_from_needle(
    needle: String,
    current_address: &Address,
    data: &ValidationData,
) -> Result<AddressKind, InvalidAddressErrorKind> {
    let (current_knot_name, current_stitches) =
        get_knot_name_and_stitches(current_address, &data.knots, &needle)?;

    let matches_stitch_in_current_knot = current_stitches.contains(&needle);
    let matches_knot = data.knots.get(&needle);
    let matches_variable = data.follow_data.variables.contains_key(&needle);

    if matches_stitch_in_current_knot {
        Ok(AddressKind::Location {
            knot: current_knot_name.to_string(),
            stitch: needle,
        })
    } else if let Some(knot_info) = matches_knot {
        Ok(AddressKind::Location {
            knot: needle,
            stitch: knot_info.default_stitch.clone(),
        })
    } else if matches_variable {
        Ok(AddressKind::GlobalVariable { name: needle })
    } else {
        Err(InvalidAddressErrorKind::UnknownAddress {
            name: needle.clone(),
        })
    }
}

/// Get the knot name and stitches from the given address.
fn get_knot_name_and_stitches(
    address: &Address,
    knots: &HashMap<String, KnotValidationInfo>,
    needle: &str,
) -> Result<(String, Vec<String>), InvalidAddressErrorKind> {
    let knot_name = address.get_knot().map_err(|_| {
        InvalidAddressErrorKind::ValidatedWithUnvalidatedAddress {
            needle: needle.to_string(),
            current_address: address.clone(),
        }
    })?;

    let KnotValidationInfo { stitches, .. } =
        knots
            .get(knot_name)
            .ok_or(InvalidAddressErrorKind::UnknownCurrentAddress {
                address: address.clone(),
            })?;

    Ok((knot_name.to_string(), stitches.keys().cloned().collect()))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::{
        consts::ROOT_KNOT_NAME,
        line::Variable,
        story::{parse::tests::read_knots_from_string, types::VariableInfo},
    };

    impl Address {
        fn from_knot(name: &str) -> Self {
            Address::Validated(AddressKind::Location {
                knot: name.to_string(),
                stitch: ROOT_KNOT_NAME.to_string(),
            })
        }

        /// Get an unvalidated address from parts
        pub fn from_parts_unchecked(knot: &str, stitch: Option<&str>) -> Self {
            let stitch_name = stitch.unwrap_or(ROOT_KNOT_NAME);

            Address::Validated(AddressKind::Location {
                knot: knot.to_string(),
                stitch: stitch_name.to_string(),
            })
        }

        pub fn variable_unchecked(name: &str) -> Self {
            Address::Validated(AddressKind::GlobalVariable {
                name: name.to_string(),
            })
        }
    }

    fn validate_address(
        address: &mut Address,
        current_location: &Address,
        data: &ValidationData,
    ) -> Result<(), InvalidAddressError> {
        let mut error = ValidationError::new();
        let mut log = Logger::default();

        address.validate(&mut error, &mut log, current_location, &().into(), data);

        if error.is_empty() {
            Ok(())
        } else {
            Err(error.invalid_address_errors[0].clone())
        }
    }

    #[test]
    fn string_representation_of_variable_addresses_is_the_address() {
        assert_eq!(
            &Address::variable_unchecked("variable").to_string(),
            "variable"
        );
    }

    #[test]
    fn string_representation_of_validated_location_is_knot_dot_stitch() {
        assert_eq!(
            &Address::from_parts_unchecked("knot", Some("stitch")).to_string(),
            "knot.stitch"
        );
    }

    #[test]
    fn string_representation_of_validated_location_with_just_knot_gets_knot_name() {
        assert_eq!(
            &Address::from_parts_unchecked("knot", None).to_string(),
            "knot"
        );
    }

    #[test]
    fn string_representation_of_raw_address_is_its_content() {
        assert_eq!(
            &Address::Raw("knot.stitch".to_string()).to_string(),
            "knot.stitch"
        );
    }

    #[test]
    fn string_representation_of_end_address_is_end() {
        assert_eq!(&Address::End.to_string(), "END");
    }

    #[test]
    fn raw_addresses_are_validated_if_they_can_parse_into_valid_knots() {
        let content = "
== tripoli
-> END

== addis_ababa
-> END
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidationData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli".to_string());

        assert!(validate_address(&mut address, &current_address, &data).is_ok());

        assert_eq!(address.get_knot().unwrap(), "tripoli");
        assert_eq!(address.get_stitch().unwrap(), "$ROOT$");
    }

    #[test]
    fn if_default_stitch_is_set_in_knot_addresses_validate_to_it() {
        let content = "
== tripoli
= cinema
-> END

== addis_ababa
-> END
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidationData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli".to_string());

        assert!(validate_address(&mut address, &current_address, &data).is_ok());

        assert_eq!(address.get_knot().unwrap(), "tripoli");
        assert_eq!(address.get_stitch().unwrap(), "cinema");
    }

    #[test]
    fn raw_addresses_are_validated_if_they_can_parse_into_valid_stitches() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
-> END

= with_family
-> END

== addis_ababa
-> END
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidationData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli.with_family".to_string());

        assert!(validate_address(&mut address, &current_address, &data).is_ok());

        assert_eq!(address.get_knot().unwrap(), "tripoli");
        assert_eq!(address.get_stitch().unwrap(), "with_family");
    }

    #[test]
    fn stitch_labels_may_be_relative_to_current_knot() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
-> END

= with_family
-> END
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidationData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("tripoli");

        let mut address = Address::Raw("with_family".to_string());

        assert!(validate_address(&mut address, &current_address, &data).is_ok());

        assert_eq!(address.get_knot().unwrap(), "tripoli");
        assert_eq!(address.get_stitch().unwrap(), "with_family");
    }

    #[test]
    fn if_knot_address_is_not_found_an_error_is_yielded() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
-> END

== addis_ababa
-> END
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidationData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("rabat");

        assert!(validate_address(
            &mut Address::Raw("addis_ababa".to_string()),
            &current_address,
            &data
        )
        .is_err());
    }

    #[test]
    fn if_address_is_poorly_formatted_an_error_is_yielded_from_validation() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidationData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("tripoli");

        assert!(validate_address(
            &mut Address::Raw("tripoli.".to_string()),
            &current_address,
            &data
        )
        .is_err());

        assert!(validate_address(
            &mut Address::Raw(".tripoli".to_string()),
            &current_address,
            &data
        )
        .is_err());
    }

    #[test]
    fn if_address_exists_as_stitch_but_in_another_knot_an_error_is_yielded() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
-> END

= cinema
-> END

== addis_ababa
You find yourself in Addis Ababa, the capital of Ethiopia.
-> END
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidationData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        assert!(validate_address(
            &mut Address::Raw("cinema".to_string()),
            &current_address,
            &data
        )
        .is_err());
    }

    #[test]
    fn if_simple_address_exists_in_variables_it_is_validated_as_global() {
        let content = "
== addis_ababa
You find yourself in Addis Ababa, the capital of Ethiopia.
-> END
";

        let knots = read_knots_from_string(content).unwrap();

        let variables = &[("counter".to_string(), Variable::Int(0))]
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, (name, var))| (name, VariableInfo::new(var, i)))
            .collect();

        let data = ValidationData::from_data(&knots, &variables);

        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("counter".to_string());
        validate_address(&mut address, &current_address, &data).unwrap();

        assert_eq!(
            address,
            Address::Validated(AddressKind::GlobalVariable {
                name: "counter".to_string()
            })
        );
    }

    #[test]
    fn done_and_end_knot_names_validate_to_special_address() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
-> END
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidationData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("");

        let mut end_address = Address::Raw("END".to_string());
        assert!(validate_address(&mut end_address, &current_address, &data).is_ok());
        assert_eq!(end_address, Address::End);

        let mut done_address = Address::Raw("DONE".to_string());
        assert!(validate_address(&mut done_address, &current_address, &data).is_ok());
        assert_eq!(done_address, Address::End);
    }

    #[test]
    fn address_from_location_is_validated() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
-> END

= cinema
-> END

== addis_ababa
You find yourself in Addis Ababa, the capital of Ethiopia.
-> END

";

        let knots = read_knots_from_string(content).unwrap();

        assert_eq!(
            Address::from_location(&"addis_ababa".into(), &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "addis_ababa".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            })
        );

        assert_eq!(
            Address::from_location(&"tripoli".into(), &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "tripoli".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            })
        );

        assert_eq!(
            Address::from_location(&Location::with_stitch("tripoli", "cinema"), &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "tripoli".to_string(),
                stitch: "cinema".to_string()
            })
        );

        assert!(Address::from_location(&"rabat".into(), &knots).is_err());
        assert!(
            Address::from_location(&Location::with_stitch("addis_ababa", "cinema"), &knots)
                .is_err()
        );
        assert!(
            Address::from_location(&Location::with_stitch("tripoli", "with_family"), &knots)
                .is_err()
        );
    }

    #[test]
    fn address_from_location_uses_default_stitch_if_set() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.

= cinema
Ah, the cinema. // We should not end up at this stitch by default

== cairo
= airport
You find yourself in Cairo, the capital of Egypt.

";

        let knots = read_knots_from_string(content).unwrap();

        assert_eq!(
            Address::from_location(&"tripoli".into(), &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "tripoli".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            })
        );

        assert_eq!(
            Address::from_location(&"cairo".into(), &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "cairo".to_string(),
                stitch: "airport".to_string()
            })
        );
    }
}
