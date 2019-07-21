use super::story::Knots;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use crate::error::InklingError;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// A verified address to a `Knot` or `Stitch` in the story.
///
/// Used to leverage the type system and ensure that functions which require complete addresses
/// get them.
pub struct Address {
    pub knot: String,
    pub stitch: String,
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
            (knot, Some(stitch)) => get_full_address(knot, stitch, knots),
            (head, None) => get_full_address_from_head(head, current_address, knots),
        }?;

        Ok(Address { knot, stitch })
    }

    pub fn from_root_knot(root_knot_name: &str, knots: &Knots) -> Result<Self, InklingError> {
        let knot = knots
            .get(root_knot_name)
            .ok_or(InklingError::InvalidAddress)?;

        Ok(Address {
            knot: root_knot_name.to_string(),
            stitch: knot.default_stitch.clone(),
        })
    }
}

fn split_address_into_parts(address: &str) -> Result<(&str, Option<&str>), InklingError> {
    if let Some(i) = address.find('.') {
        let knot = address.get(..i).unwrap();
        let stitch = address.get(i + 1..).ok_or(InklingError::InvalidAddress)?;

        Ok((knot, Some(stitch)))
    } else {
        Ok((address, None))
    }
}

fn get_full_address(
    knot: &str,
    stitch: &str,
    knots: &Knots,
) -> Result<(String, String), InklingError> {
    let target_knot = knots.get(knot).ok_or(InklingError::InvalidAddress)?;

    if target_knot.stitches.contains_key(stitch) {
        Ok((knot.to_string(), stitch.to_string()))
    } else {
        Err(InklingError::InvalidAddress)
    }
}

fn get_full_address_from_head(
    head: &str,
    current_address: &Address,
    knots: &Knots,
) -> Result<(String, String), InklingError> {
    let current_knot = knots
        .get(&current_address.knot)
        .ok_or(InklingError::InvalidAddress)?;

    if current_knot.stitches.contains_key(head) {
        Ok((current_address.knot.clone(), head.to_string()))
    } else {
        let target_knot = knots.get(head).ok_or(InklingError::InvalidAddress)?;
        Ok((head.to_string(), target_knot.default_stitch.clone()))
    }
}

#[cfg(test)]
pub mod tests {
    use super::super::parse::read_knots_from_string;
    use super::*;

    use crate::consts::ROOT_KNOT_NAME;

    impl Address {
        fn from_knot(name: &str) -> Self {
            Address {
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
            Address {
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
            Address {
                knot: "knot_one".into(),
                stitch: "stitch".into()
            }
        );

        let address = Address::from_target_address("knot_two", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address {
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
            Address {
                knot: "knot_one".into(),
                stitch: "stitch_one".into()
            }
        );

        let address = Address::from_target_address("stitch_two", &current_address, &knots).unwrap();
        assert_eq!(
            address,
            Address {
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
            Address {
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
            Address {
                knot: "paris".into(),
                stitch: "opera".into()
            }
        );
    }
}
