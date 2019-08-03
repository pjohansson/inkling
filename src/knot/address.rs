//! Validated addresses to content in a story.

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use crate::{
    consts::{DONE_KNOT, END_KNOT},
    error::{InternalError, InvalidAddressError},
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
    Validated { knot: String, stitch: String },
    /// This string-formatted address has not yet been validated.
    Raw(String),
    /// Divert address to mark that a story is finished.
    End,
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

        Ok(Address::Validated {
            knot: root_knot_name.to_string(),
            stitch: knot.default_stitch.clone(),
        })
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
            Ok(Address::Validated {
                knot: knot_name.to_string(),
                stitch: stitch_name.to_string(),
            })
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
            Address::Validated { knot, .. } => Ok(knot),
            _ => Err(InternalError::UseOfUnvalidatedAddress {
                address: self.clone(),
            }),
        }
    }

    /// Get the stitch name of a validateed address.
    pub fn get_stitch(&self) -> Result<&str, InternalError> {
        match self {
            Address::Validated { stitch, .. } => Ok(stitch),
            _ => Err(InternalError::UseOfUnvalidatedAddress {
                address: self.clone(),
            }),
        }
    }

    /// Get knot and stitch names from a validated address.
    pub fn get_knot_and_stitch(&self) -> Result<(&str, &str), InternalError> {
        match self {
            Address::Validated { knot, stitch } => Ok((knot, stitch)),
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
                let (knot, stitch) = match split_address_into_parts(target.trim())? {
                    (knot, Some(stitch)) => get_full_address(knot, stitch, &data.knot_structure)?,
                    (head, None) => get_full_address_from_head(head, current_address, data)?,
                };

                *self = Address::Validated { knot, stitch };
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
fn split_address_into_parts(address: &str) -> Result<(&str, Option<&str>), InvalidAddressError> {
    if let Some(i) = address.find('.') {
        let knot = address.get(..i).unwrap();

        let stitch = address.get(i + 1..).ok_or(InvalidAddressError::BadFormat {
            line: address.to_string(),
        })?;

        Ok((knot, Some(stitch)))
    } else {
        Ok((address, None))
    }
}

/// Verify and return the full address to a node.
fn get_full_address(
    knot_name: &str,
    stitch_name: &str,
    knot_structure: &HashMap<String, (String, Vec<String>)>,
) -> Result<(String, String), InvalidAddressError> {
    let (_, stitches) = knot_structure
        .get(knot_name)
        .ok_or(InvalidAddressError::UnknownKnot {
            knot_name: knot_name.to_string(),
        })?;

    if stitches.contains(&stitch_name.to_string()) {
        Ok((knot_name.to_string(), stitch_name.to_string()))
    } else {
        Err(InvalidAddressError::UnknownStitch {
            knot_name: knot_name.to_string(),
            stitch_name: stitch_name.to_string(),
        })
    }
}

/// Return the full address from either an internal address or knot name.
///
/// Internal addresses are relative to the current knot. If one is found in the current knot,
/// the knot name and the address is returned. Otherwise the default stitch from a knot
/// with the name is returned.
fn get_full_address_from_head(
    needle: &str,
    current_address: &Address,
    data: &ValidateAddressData,
) -> Result<(String, String), InvalidAddressError> {
    let current_knot_name = current_address.get_knot().map_err(|_| {
        InvalidAddressError::ValidatedWithUnvalidatedAddress {
            needle: needle.to_string(),
            current_address: current_address.clone(),
        }
    })?;

    let (_, current_knot_stitches) =
        data.knot_structure
            .get(current_knot_name)
            .ok_or(InvalidAddressError::UnknownCurrentAddress {
                address: current_address.clone(),
            })?;

    if current_knot_stitches.contains(&needle.to_string()) {
        Ok((current_knot_name.to_string(), needle.to_string()))
    } else if let Some((default_stitch, _)) = data.knot_structure.get(needle) {
        Ok((needle.to_string(), default_stitch.clone()))
    } else {
        Err(InvalidAddressError::UnknownKnot { knot_name: needle.to_string() })
    }
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
                    let current_address = Address::Validated {
                        knot: knot_name.clone(),
                        stitch: stitch_name.clone(),
                    };

                    stitch.root.validate(&current_address, &validation_data)
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::{consts::ROOT_KNOT_NAME, story::read_knots_from_string};

    impl Address {
        fn from_knot(name: &str) -> Self {
            Address::Validated {
                knot: name.to_string(),
                stitch: String::new(),
            }
        }

        /// Get an unvalidated address from parts
        pub fn from_parts_unchecked(knot: &str, stitch: Option<&str>) -> Self {
            let stitch_name = stitch.unwrap_or(ROOT_KNOT_NAME);
            Address::Validated {
                knot: knot.to_string(),
                stitch: stitch_name.to_string(),
            }
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

        let (_, knots) = read_knots_from_string(content).unwrap();

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

        let (_, knots) = read_knots_from_string(content).unwrap();

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

        let (_, knots) = read_knots_from_string(content).unwrap();
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

        let (_, knots) = read_knots_from_string(content).unwrap();
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

        let (_, knots) = read_knots_from_string(content).unwrap();
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

        let (_, knots) = read_knots_from_string(content).unwrap();
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

        let (_, knots) = read_knots_from_string(content).unwrap();
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

        let (_, knots) = read_knots_from_string(content).unwrap();
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

        let (_, knots) = read_knots_from_string(content).unwrap();
        let data = ValidateAddressData::from_data(&knots, &HashMap::new());

        let current_address = Address::from_knot("addis_ababa");

        assert!(Address::Raw("cinema".to_string())
            .validate(&current_address, &data)
            .is_err());
    }

    #[test]
    fn done_and_end_knot_names_validate_to_special_address() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
-> END
";

        let (_, knots) = read_knots_from_string(content).unwrap();
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

        let (_, knots) = read_knots_from_string(content).unwrap();

        assert_eq!(
            Address::from_parts("addis_ababa", None, &knots).unwrap(),
            Address::Validated {
                knot: "addis_ababa".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            }
        );

        assert_eq!(
            Address::from_parts("tripoli", None, &knots).unwrap(),
            Address::Validated {
                knot: "tripoli".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            }
        );

        assert_eq!(
            Address::from_parts("tripoli", Some("cinema"), &knots).unwrap(),
            Address::Validated {
                knot: "tripoli".to_string(),
                stitch: "cinema".to_string()
            }
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

        let (_, knots) = read_knots_from_string(content).unwrap();

        assert_eq!(
            Address::from_parts("tripoli", None, &knots).unwrap(),
            Address::Validated {
                knot: "tripoli".to_string(),
                stitch: ROOT_KNOT_NAME.to_string()
            }
        );

        assert_eq!(
            Address::from_parts("cairo", None, &knots).unwrap(),
            Address::Validated {
                knot: "cairo".to_string(),
                stitch: "airport".to_string()
            }
        );
    }
}
