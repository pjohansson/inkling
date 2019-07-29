//! Validated addresses to nodes or content in a story.

use super::story::Knots;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use crate::{
    consts::{DONE_KNOT, END_KNOT},
    error::{InklingError, InvalidAddressError, StackError},
};

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
    /// Get the full address from a target.
    ///
    /// The given address may be to a `Knot`, in which case its default `Stitch` is used.
    ///
    /// The given address may be internal to the `Knot` specified by `current_address`,
    /// in which case the full address is returned.
    ///
    /// For example, if we are currently in a knot with name `helsinki` and want to move to
    /// a stitch within it with the name `date_with_kielo`, this function can be given
    /// `date_with_kielo` and return the full address `helsinki` and `date_with_kielo`.
    pub fn from_target_address(
        target: &str,
        current_address: &Address,
        knots: &Knots,
    ) -> Result<Self, InklingError> {
        let (knot, stitch) = match split_address_into_parts(target.trim())? {
            (knot, Some(stitch)) => get_full_address(knot, stitch, knots)?,
            (head, None) => get_full_address_from_head(head, current_address, knots)?,
        };

        Ok(Address::Validated { knot, stitch })
    }

    /// Return an address from a string that is just a knot name.
    ///
    /// The knot name is verified as present in the `Knots` set. The `Stitch` is set
    /// as the default for the found `Knot`.
    pub fn from_root_knot(root_knot_name: &str, knots: &Knots) -> Result<Self, InklingError> {
        let knot = knots.get(root_knot_name).ok_or(StackError::NoRootKnot {
            knot_name: root_knot_name.to_string(),
        })?;

        Ok(Address::Validated {
            knot: root_knot_name.to_string(),
            stitch: knot.default_stitch.clone(),
        })
    }

    pub fn get_knot(&self) -> &str {
        match self {
            Address::Validated { knot, .. } => knot,
            Address::End => panic!("tried to get `Knot` name from a divert to `End`"),
            _ => panic!("tried to get `Knot` name from an unvalidated `Address`"),
        }
    }

    pub fn get_stitch(&self) -> &str {
        match self {
            Address::Validated { stitch, .. } => stitch,
            Address::End => panic!("tried to get `Stitch` name from a divert to `End`"),
            _ => panic!("tried to get `Stitch` name from an unvalidated `Address`"),
        }
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
    let current_knot = knots.get(current_address.get_knot()).ok_or(
        InvalidAddressError::UnknownCurrentAddress {
            address: current_address.clone(),
        },
    )?;

    if current_knot.stitches.contains_key(head) {
        Ok((current_address.get_knot().to_string(), head.to_string()))
    } else {
        let target_knot = knots.get(head).ok_or(InvalidAddressError::UnknownKnot {
            knot_name: head.to_string(),
        })?;
        Ok((head.to_string(), target_knot.default_stitch.clone()))
    }
}

fn validate_addresses_in_knots(knots: &mut Knots) -> Result<(), InvalidAddressError> {
    unimplemented!();
}

pub trait ValidateAddresses {
    fn validate(
        &mut self,
        current_address: &Address,
        knots: &Knots,
    ) -> Result<(), InvalidAddressError>;
    fn all_addresses_are_valid(&self) -> bool;
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
    fn address_from_complete_address_return_the_same_address() {
        let content = "
== knot
= stitch
Line one.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("knot");

        let address =
            Address::from_target_address("knot.stitch", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address::Validated {
                knot: "knot".into(),
                stitch: "stitch".into()
            }
        );
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
    fn address_from_internal_address_returns_full_address() {
        let content = "
== knot_one

= stitch_one
Line one.

= stitch_two
Line two.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("knot_one");

        let address = Address::from_target_address("stitch_one", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address::Validated {
                knot: "knot_one".into(),
                stitch: "stitch_one".into()
            }
        );

        let address = Address::from_target_address("stitch_two", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address::Validated {
                knot: "knot_one".into(),
                stitch: "stitch_two".into()
            }
        );
    }

    #[test]
    fn address_from_internal_address_always_prioritizes_internal_stitches_over_knots() {
        let content = "
== paris

= opera
Line one.

== opera
Line two.

== hamburg

= opera
Line three.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("hamburg");

        let address = Address::from_target_address("opera", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address::Validated {
                knot: "hamburg".into(),
                stitch: "opera".into()
            }
        );
    }

    #[test]
    fn constructing_address_returns_error_if_target_does_not_exist() {
        let content = "
== paris

= opera
Line one.

== opera
Line two.

== hamburg

= opera
Line three.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("paris");

        assert!(Address::from_target_address("stockholm", &current_address, &knots).is_err());
        assert!(Address::from_target_address("hamburg.cinema", &current_address, &knots).is_err());
    }

    #[test]
    fn constructing_address_returns_error_if_either_knot_or_stitch_part_is_missing() {
        let content = "
== paris

= opera
Line one.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("paris");

        assert!(Address::from_target_address(".", &current_address, &knots).is_err());
        assert!(Address::from_target_address("paris.", &current_address, &knots).is_err());
        assert!(Address::from_target_address(".opera", &current_address, &knots).is_err());
    }

    #[test]
    fn constructing_address_returns_error_if_current_address_is_invalid() {
        let content = "
== paris

= opera
Line one.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("hamburg");

        assert!(Address::from_target_address("paris", &current_address, &knots).is_err());
    }

    #[test]
    fn constructing_address_trims_whitespace() {
        let content = "
== paris

= opera
Line one.
";

        let (_, knots) = read_knots_from_string(content).unwrap();
        let current_address = Address::from_knot("hamburg");

        let address =
            Address::from_target_address(" paris.opera ", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address::Validated {
                knot: "paris".into(),
                stitch: "opera".into()
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

        assert_eq!(address.get_knot(), "tripoli");
        assert_eq!(address.get_stitch(), "$ROOT$");
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

        assert_eq!(address.get_knot(), "tripoli");
        assert_eq!(address.get_stitch(), "cinema");
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

        assert_eq!(address.get_knot(), "tripoli");
        assert_eq!(address.get_stitch(), "with_family");
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

        assert_eq!(address.get_knot(), "tripoli");
        assert_eq!(address.get_stitch(), "with_family");
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
