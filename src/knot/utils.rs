use crate::{
    error::{InternalError, StackError},
    follow::FollowData,
    knot::{Address, KnotSet, Stitch},
};

use std::collections::HashMap;

#[allow(dead_code)]
/// Return a reference to the `Stitch` at the target address.
pub fn get_stitch<'a>(address: &Address, knots: &'a KnotSet) -> Result<&'a Stitch, InternalError> {
    let (knot_name, stitch_name) = address.get_knot_and_stitch()?;

    knots
        .get(knot_name)
        .and_then(|knot| knot.stitches.get(stitch_name))
        .ok_or(
            StackError::BadAddress {
                address: address.clone(),
            }
            .into(),
        )
}

/// Return a mutable reference to the `Stitch` at the target address.
pub fn get_mut_stitch<'a>(
    address: &Address,
    knots: &'a mut KnotSet,
) -> Result<&'a mut Stitch, InternalError> {
    let (knot_name, stitch_name) = address.get_knot_and_stitch()?;

    knots
        .get_mut(knot_name)
        .and_then(|knot| knot.stitches.get_mut(stitch_name))
        .ok_or(
            StackError::BadAddress {
                address: address.clone(),
            }
            .into(),
        )
}

pub fn get_num_visited(address: &Address, data: &FollowData) -> Result<u32, InternalError> {
    let (knot_name, stitch_name) = address.get_knot_and_stitch()?;

    data.knot_visit_counts
        .get(knot_name)
        .and_then(|knot| knot.get(stitch_name).copied())
        .ok_or(
            StackError::BadAddress {
                address: address.clone(),
            }
            .into(),
        )
}

pub fn increment_num_visited(
    address: &Address,
    data: &mut FollowData,
) -> Result<(), InternalError> {
    let (knot_name, stitch_name) = address.get_knot_and_stitch()?;

    data.knot_visit_counts
        .get_mut(knot_name)
        .and_then(|knot| knot.get_mut(stitch_name).map(|count| *count += 1))
        .ok_or(
            StackError::BadAddress {
                address: address.clone(),
            }
            .into(),
        )
}

pub fn get_empty_knot_counts(knots: &KnotSet) -> HashMap<String, HashMap<String, u32>> {
    knots
        .iter()
        .map(|(knot_name, knot)| {
            let empty = knot
                .stitches
                .iter()
                .map(|(stitch_name, _)| (stitch_name.clone(), 0))
                .collect();

            (knot_name.clone(), empty)
        })
        .collect()
}
