//! Validated addresses to nodes or content in a story.

use super::story::Knots;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use crate::{
    consts::{DONE_KNOT, END_KNOT},
    error::{InternalError, InvalidAddressError},
    knot::{Knot, Stitch},
    node::RootNodeBuilder,
};

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
        knots: &Knots,
    ) -> Result<(), InvalidAddressError>;

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
    /// The knot name is verified as present in the `Knots` set. The `Stitch` is set
    /// as the default for the found `Knot`.
    pub fn from_root_knot(
        root_knot_name: &str,
        knots: &Knots,
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

    #[cfg(test)]
    /// Get a validated address from a string.
    pub fn from_target_address(
        target: &str,
        current_address: &Address,
        knots: &Knots,
    ) -> Result<Self, InvalidAddressError> {
        let mut address = Address::Raw(target.to_string());
        address.validate(current_address, knots).map(|_| address)
    }
}

impl ValidateAddresses for Address {
    fn validate(
        &mut self,
        current_address: &Address,
        knots: &Knots,
    ) -> Result<(), InvalidAddressError> {
        match self {
            Address::Raw(ref target) if target == DONE_KNOT || target == END_KNOT => {
                *self = Address::End;
            }
            Address::Raw(ref target) => {
                let (knot, stitch) = match split_address_into_parts(target.trim())? {
                    (knot, Some(stitch)) => get_full_address(knot, stitch, knots)?,
                    (head, None) => get_full_address_from_head(head, current_address, knots)?,
                };

                *self = Address::Validated { knot, stitch };
            }
            _ => (),
        }

        Ok(())
    }

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
    knot: &str,
    stitch: &str,
    knots: &Knots,
) -> Result<(String, String), InvalidAddressError> {
    let target_knot = knots.get(knot).ok_or(InvalidAddressError::UnknownKnot {
        knot_name: knot.to_string(),
    })?;

    if target_knot.stitches.contains_key(stitch) {
        Ok((knot.to_string(), stitch.to_string()))
    } else {
        Err(InvalidAddressError::UnknownStitch {
            knot_name: knot.to_string(),
            stitch_name: stitch.to_string(),
        })
    }
}

/// Return the full address from either an internal address or knot name.
///
/// Internal addresses are relative to the current knot. If one is found in the current knot,
/// the knot name and the address is returned. Otherwise the default stitch from a knot
/// with the name is returned.
fn get_full_address_from_head(
    head: &str,
    current_address: &Address,
    knots: &Knots,
) -> Result<(String, String), InvalidAddressError> {
    let current_knot_name = current_address.get_knot().map_err(|_| {
        InvalidAddressError::ValidatedWithUnvalidatedAddress {
            needle: head.to_string(),
            current_address: current_address.clone(),
        }
    })?;

    let current_knot =
        knots
            .get(current_knot_name)
            .ok_or(InvalidAddressError::UnknownCurrentAddress {
                address: current_address.clone(),
            })?;

    if current_knot.stitches.contains_key(head) {
        Ok((current_knot_name.to_string(), head.to_string()))
    } else {
        let target_knot = knots.get(head).ok_or(InvalidAddressError::UnknownKnot {
            knot_name: head.to_string(),
        })?;
        Ok((head.to_string(), target_knot.default_stitch.clone()))
    }
}

/// Validate all addresses in knots using the `ValidateAddresses` trait.
pub fn validate_addresses_in_knots(knots: &mut Knots) -> Result<(), InvalidAddressError> {
    let empty_knots = get_empty_knot_map(knots);

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

                    stitch.root.validate(&current_address, &empty_knots)
                })
                .collect()
        })
        .collect()
}

/// Return an empty copy of the `Knots` set.
fn get_empty_knot_map(knots: &Knots) -> Knots {
    knots
        .iter()
        .map(|(knot_name, knot)| {
            let empty_stitches = knot
                .stitches
                .keys()
                .map(|stitch_name| {
                    let empty_stitch = Stitch {
                        root: RootNodeBuilder::new().build(),
                        stack: Vec::new(),
                        num_visited: 0,
                    };

                    (stitch_name.clone(), empty_stitch)
                })
                .collect();

            let empty_knot = Knot {
                default_stitch: knot.default_stitch.clone(),
                stitches: empty_stitches,
            };

            (knot_name.clone(), empty_knot)
        })
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::super::parse::read_knots_from_string;
    use super::*;

    use crate::consts::ROOT_KNOT_NAME;

    impl Address {
        fn from_knot(name: &str) -> Self {
            Address::Validated {
                knot: name.to_string(),
                stitch: String::new(),
            }
        }
    }

    #[test]
    fn creating_empty_knots_from_base_conserves_default_stitch_names() {
        let content = "
== tripoli
= cinema
-> END
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let empty_knot = get_empty_knot_map(&knots);

        assert_eq!(&empty_knot.get("tripoli").unwrap().default_stitch, "cinema");
    }

    #[test]
    fn address_from_knot_address_returns_knot_with_default_stitch() {
        let content = "
== knot_one
= stitch
Line one.

== knot_two
Line two.
= stitch
Line three.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("knot_one");

        let address = Address::from_target_address("knot_one", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address::Validated {
                knot: "knot_one".into(),
                stitch: "stitch".into()
            }
        );

        let address = Address::from_target_address("knot_two", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address::Validated {
                knot: "knot_two".into(),
                stitch: ROOT_KNOT_NAME.into()
            }
        );
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
        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli".to_string());

        assert!(address.validate(&current_address, &knots).is_ok());

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
        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli".to_string());

        assert!(address.validate(&current_address, &knots).is_ok());

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
        let current_address = Address::from_knot("addis_ababa");

        let mut address = Address::Raw("tripoli.with_family".to_string());

        assert!(address.validate(&current_address, &knots).is_ok());

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
        let current_address = Address::from_knot("tripoli");

        let mut address = Address::Raw("with_family".to_string());

        assert!(address.validate(&current_address, &knots).is_ok());

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
        let current_address = Address::from_knot("rabat");

        assert!(Address::Raw("addis_ababa".to_string())
            .validate(&current_address, &knots)
            .is_err());
    }

    #[test]
    fn if_address_is_poorly_formatted_an_error_is_yielded_from_validation() {
        let content = "
== tripoli
You find yourself in Tripoli, the capital of Libya.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("tripoli");

        assert!(Address::Raw("tripoli.".to_string())
            .validate(&current_address, &knots)
            .is_err());

        assert!(Address::Raw(".tripoli".to_string())
            .validate(&current_address, &knots)
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
        let current_address = Address::from_knot("addis_ababa");

        assert!(Address::Raw("cinema".to_string())
            .validate(&current_address, &knots)
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
        let current_address = Address::from_knot("");

        let mut end_address = Address::Raw("END".to_string());
        assert!(end_address.validate(&current_address, &knots).is_ok());
        assert_eq!(end_address, Address::End);

        let mut done_address = Address::Raw("DONE".to_string());
        assert!(done_address.validate(&current_address, &knots).is_ok());
        assert_eq!(done_address, Address::End);
    }
}
