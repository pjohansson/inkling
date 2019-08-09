//! Validated addresses to content in a story.

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use crate::{
    consts::{DONE_KNOT, END_KNOT},
    error::{parse::address::InvalidAddressError, InternalError},
    follow::FollowData,
    knot::KnotSet,
    line::Variable,
};

use std::collections::{HashMap, HashSet};

pub struct ValidateAddressData {
    /// Set of knots in story, each with their default stitch and list of stitches.
    knot_structure: HashMap<String, (String, Vec<String>)>,
    /// Set of global variables in story.
    variables: HashSet<String>,
}

impl ValidateAddressData {
    fn from_data(knots: &KnotSet, variable_set: &HashMap<String, Variable>) -> Self {
        let knot_structure = knots
            .iter()
            .map(|(knot_name, knot)| {
                let stitches = knot.stitches.keys().cloned().collect();

                (knot_name.clone(), (knot.default_stitch.clone(), stitches))
            })
            .collect();

        let variables = variable_set.keys().cloned().collect();

        ValidateAddressData {
            knot_structure,
            variables,
        }
    }
}

/// Trait for validating `Address` objects.
///
/// Meant to be implemented recursively for all relevant items. Any new item that has an
/// address should implement this trait and ensure that a parent item calls this function
/// on it when itself called.
///
/// At the end of a recursively called chain of objects containing addresses somewhere
/// there should be the actual address that will be verified.
pub trait ValidateAddresses {
    /// Validate any addresses belonging to this item or their children.
    fn validate(
        &mut self,
        current_address: &Address,
        data: &ValidateAddressData,
    ) -> Result<(), InvalidAddressError>;

    #[cfg(test)]
    /// Assert that all addresses are valid.
    fn all_addresses_are_valid(&self) -> bool;
}

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

impl Address {
    /// Return an address from a string that is just a knot name.
    ///
    /// The knot name is verified as present in the `KnotSet` set. The `Stitch` is set
    /// as the default for the found `Knot`.
    pub fn from_root_knot(
        root_knot_name: &str,
        knots: &KnotSet,
    ) -> Result<Self, InvalidAddressError> {
        let knot = knots
            .get(root_knot_name)
            .ok_or(InvalidAddressError::UnknownKnot {
                knot_name: root_knot_name.to_string(),
            })?;

        Ok(Address::Validated(AddressKind::Location {
            knot: root_knot_name.to_string(),
            stitch: knot.default_stitch.clone(),
        }))
    }

    pub fn from_parts(
        knot_name: &str,
        stitch_name: Option<&str>,
        knots: &KnotSet,
    ) -> Result<Self, InvalidAddressError> {
        let knot = knots
            .get(knot_name)
            .ok_or(InvalidAddressError::UnknownKnot {
                knot_name: knot_name.to_string(),
            })?;

        let stitch_name = stitch_name.unwrap_or(&knot.default_stitch);

        if knot.stitches.contains_key(stitch_name) {
            Ok(Address::Validated(AddressKind::Location {
                knot: knot_name.to_string(),
                stitch: stitch_name.to_string(),
            }))
        } else {
            Err(InvalidAddressError::UnknownStitch {
                knot_name: knot_name.to_string(),
                stitch_name: stitch_name.to_string(),
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
}

impl ValidateAddresses for Address {
    fn validate(
        &mut self,
        current_address: &Address,
        data: &ValidateAddressData,
    ) -> Result<(), InvalidAddressError> {
        match self {
            Address::Raw(ref target) if target == DONE_KNOT || target == END_KNOT => {
                *self = Address::End;
            }
            Address::Raw(ref target) => {
                let address = match split_address_into_parts(target.trim())? {
                    (knot, Some(stitch)) => {
                        get_location_from_parts(knot, stitch, &data.knot_structure)?
                    }
                    (needle, None) => get_address_from_needle(needle, current_address, data)?,
                }
                .into();

                *self = address;
            }
            Address::Validated { .. } | Address::End => (),
        }

        Ok(())
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        match self {
            Address::Validated { .. } | Address::End => true,
            Address::Raw(..) => false,
        }
    }
}

/// Split an address into constituent parts if possible.
///
/// The split is done at a dot ('.') marker. If one exists, split at it and return the parts.
/// Otherwise return the entire string.
fn split_address_into_parts(
    address: &str,
) -> Result<(String, Option<String>), InvalidAddressError> {
    if let Some(i) = address.find('.') {
        let knot = address.get(..i).unwrap();

        let stitch = address.get(i + 1..).ok_or(InvalidAddressError::BadFormat {
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
    knot_structure: &HashMap<String, (String, Vec<String>)>,
) -> Result<AddressKind, InvalidAddressError> {
    let (_, stitches) = knot_structure
        .get(&knot_name)
        .ok_or(InvalidAddressError::UnknownKnot {
            knot_name: knot_name.clone(),
        })?;

    if stitches.contains(&stitch_name) {
        Ok(AddressKind::Location {
            knot: knot_name,
            stitch: stitch_name,
        })
    } else {
        Err(InvalidAddressError::UnknownStitch {
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
    data: &ValidateAddressData,
) -> Result<AddressKind, InvalidAddressError> {
    let (current_knot_name, current_stitches) =
        get_knot_name_and_stitches(current_address, &data.knot_structure, &needle)?;

    let matches_stitch_in_current_knot = current_stitches.contains(&needle);
    let matches_knot = data.knot_structure.get(&needle);
    let matches_variable = data.variables.contains(&needle);

    if matches_stitch_in_current_knot {
        Ok(AddressKind::Location {
            knot: current_knot_name.to_string(),
            stitch: needle,
        })
    } else if let Some((default_stitch, _)) = matches_knot {
        Ok(AddressKind::Location {
            knot: needle,
            stitch: default_stitch.clone(),
        })
    } else if matches_variable {
        Ok(AddressKind::GlobalVariable { name: needle })
    } else {
        Err(InvalidAddressError::UnknownAddress {
            name: needle.clone(),
        })
    }
}

/// Get the knot name and stitches from the given address.
fn get_knot_name_and_stitches<'a>(
    address: &Address,
    knot_structure: &'a HashMap<String, (String, Vec<String>)>,
    needle: &str,
) -> Result<(String, &'a Vec<String>), InvalidAddressError> {
    let knot_name =
        address
            .get_knot()
            .map_err(|_| InvalidAddressError::ValidatedWithUnvalidatedAddress {
                needle: needle.to_string(),
                current_address: address.clone(),
            })?;

    let (_, stitches) =
        knot_structure
            .get(knot_name)
            .ok_or(InvalidAddressError::UnknownCurrentAddress {
                address: address.clone(),
            })?;

    Ok((knot_name.to_string(), stitches))
}

/// Validate all addresses in knots using the `ValidateAddresses` trait.
pub fn validate_addresses_in_knots(
    knots: &mut KnotSet,
    data: &FollowData,
) -> Result<(), InvalidAddressError> {
    let validation_data = ValidateAddressData::from_data(knots, &data.variables);

    knots
        .iter_mut()
        .map(|(knot_name, knot)| {
            knot.stitches
                .iter_mut()
                .map(|(stitch_name, stitch)| {
                    let current_address = Address::Validated(AddressKind::Location {
                        knot: knot_name.clone(),
                        stitch: stitch_name.clone(),
                    });

                    stitch.root.validate(&current_address, &validation_data)
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::{consts::ROOT_KNOT_NAME, story::parse::tests::read_knots_from_string};

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

        let knots = read_knots_from_string(content).unwrap();

        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        assert_eq!(data.knot_structure.len(), 2);

        let (tripoli_default, _) = data.knot_structure.get("tripoli").unwrap();
        let (addis_ababa_default, _) = data.knot_structure.get("addis_ababa").unwrap();

        assert_eq!(tripoli_default.as_str(), "cinema");
        assert_eq!(addis_ababa_default.as_str(), ROOT_KNOT_NAME);
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

        let knots = read_knots_from_string(content).unwrap();

        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let (_, tripoli_stitches) = data.knot_structure.get("tripoli").unwrap();
        let (_, addis_ababa_stitches) = data.knot_structure.get("addis_ababa").unwrap();

        assert_eq!(tripoli_stitches.len(), 2);
        assert!(tripoli_stitches.contains(&"cinema".to_string()));
        assert!(tripoli_stitches.contains(&"with_family".to_string()));

        assert_eq!(addis_ababa_stitches.len(), 2);
        assert!(addis_ababa_stitches.contains(&ROOT_KNOT_NAME.to_string()));
        assert!(addis_ababa_stitches.contains(&"with_family".to_string()));
    }

    #[test]
    fn creating_validation_data_sets_variable_names() {
        let mut variables = HashMap::new();

        variables.insert("counter".to_string(), Variable::Int(1));
        variables.insert("health".to_string(), Variable::Float(75.0));

        let data = ValidateAddressData::from_data(&HashMap::new(), &variables);

        assert_eq!(data.variables.len(), 2);
        assert!(data.variables.contains("counter"));
        assert!(data.variables.contains("health"));
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
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli".to_string());

        assert!(address.validate(&current_address, &data).is_ok());

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
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli".to_string());

        assert!(address.validate(&current_address, &data).is_ok());

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
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli.with_family".to_string());

        assert!(address.validate(&current_address, &data).is_ok());

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
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("tripoli");

        let mut address = Address::Raw("with_family".to_string());

        assert!(address.validate(&current_address, &data).is_ok());

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
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("rabat");

        assert!(Address::Raw("addis_ababa".to_string())
            .validate(&current_address, &data)
            .is_err());
    }

    #[test]
    fn if_address_is_poorly_formatted_an_error_is_yielded_from_validation() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
";

        let knots = read_knots_from_string(content).unwrap();
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("tripoli");

        assert!(Address::Raw("tripoli.".to_string())
            .validate(&current_address, &data)
            .is_err());

        assert!(Address::Raw(".tripoli".to_string())
            .validate(&current_address, &data)
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
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        assert!(Address::Raw("cinema".to_string())
            .validate(&current_address, &data)
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
            .into_iter()
            .cloned()
            .collect();

        let data = ValidateAddressData::from_data(&knots, &variables);

        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("counter".to_string());
        address.validate(&current_address, &data).unwrap();

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
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("");

        let mut end_address = Address::Raw("END".to_string());
        assert!(end_address.validate(&current_address, &data).is_ok());
        assert_eq!(end_address, Address::End);

        let mut done_address = Address::Raw("DONE".to_string());
        assert!(done_address.validate(&current_address, &data).is_ok());
        assert_eq!(done_address, Address::End);
    }

    #[test]
    fn address_can_be_validated_from_parts() {
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
            Address::from_parts("addis_ababa", None, &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "addis_ababa".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            })
        );

        assert_eq!(
            Address::from_parts("tripoli", None, &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "tripoli".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            })
        );

        assert_eq!(
            Address::from_parts("tripoli", Some("cinema"), &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "tripoli".to_string(),
                stitch: "cinema".to_string()
            })
        );

        assert!(Address::from_parts("rabat", None, &knots).is_err());
        assert!(Address::from_parts("addis_ababa", Some("cinema"), &knots).is_err());
        assert!(Address::from_parts("tripoli", Some("with_family"), &knots).is_err());
    }

    #[test]
    fn if_a_default_stitch_is_set_for_the_knot_it_is_used() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
-> END

= cinema
-> END

== cairo
= airport
You find yourself in Cairo, the capital of Egypt.
-> END

";

        let knots = read_knots_from_string(content).unwrap();

        assert_eq!(
            Address::from_parts("tripoli", None, &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "tripoli".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            })
        );

        assert_eq!(
            Address::from_parts("cairo", None, &knots).unwrap(),
            Address::Validated(AddressKind::Location {
                knot: "cairo".to_string(),
                stitch: "airport".to_string()
            })
        );
    }
}
