//! Errors from parsing stories, knots, stitches and lines.

use std::{error::Error, fmt};

use crate::knot::Address;

impl Error for InvalidAddressError {}

#[derive(Clone, Debug)]
/// A divert (or other address) in the story is invalid.
pub enum InvalidAddressError {
    /// The address is not formatted correctly.
    BadFormat { line: String },
    /// The address does not reference a knot, stitch or variable in the story.
    UnknownAddress { name: String },
    /// Tried to validate an address but the given current knot did not exist in the system.
    UnknownCurrentAddress { address: Address },
    /// The address references a `Knot` that is not in the story.
    UnknownKnot { knot_name: String },
    /// The address references a `Stitch` that is not present in the current `Knot`.
    UnknownStitch {
        knot_name: String,
        stitch_name: String,
    },
    /// Tried to validate an address using an unvalidated current address.
    ValidatedWithUnvalidatedAddress {
        needle: String,
        current_address: Address,
    },
}

impl fmt::Display for InvalidAddressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use InvalidAddressError::*;

        write!(f, "Encountered an invalid address: ")?;

        match self {
            BadFormat { line } => write!(f, "address was incorrectly formatted ('{}')", line),
            UnknownAddress { name } => write!(
                f,
                "could not find knot or variable with name '{}' in the story",
                name
            ),
            UnknownCurrentAddress { address } => write!(
                f,
                "during validation an address '{:?}' that is not in the system was used as
                 a current address",
                address
            ),
            UnknownKnot { knot_name } => {
                write!(f, "no knot with name '{}' in the story", knot_name)
            }
            UnknownStitch {
                knot_name,
                stitch_name,
            } => write!(
                f,
                "no stitch with name '{}' in knot '{}'",
                stitch_name, knot_name
            ),
            ValidatedWithUnvalidatedAddress {
                needle,
                current_address,
            } => write!(
                f,
                "during validating the raw address '{}' an unvalidated address '{:?}' was used",
                needle, current_address
            ),
        }
    }
}
