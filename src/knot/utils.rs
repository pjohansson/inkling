use crate::{
    error::{InternalError, StackError},
    follow::FollowData,
    knot::{Address, KnotSet, Stitch},
};

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
