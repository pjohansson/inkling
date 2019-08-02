use crate::{
    error::{InklingError, InternalError, StackError},
    knot::{Address, KnotSet, Stitch},
};

/// Return a reference to the `Stitch` at the target address.
pub fn get_stitch<'a>(target: &Address, knots: &'a KnotSet) -> Result<&'a Stitch, InternalError> {
    let knot_name = target.get_knot()?;
    let stitch_name = target.get_stitch()?;

    knots
        .get(knot_name)
        .and_then(|knot| knot.stitches.get(stitch_name))
        .ok_or(
            StackError::BadAddress {
                address: target.clone(),
            }
            .into(),
        )
}

/// Return a mutable reference to the `Stitch` at the target address.
pub fn get_mut_stitch<'a>(
    target: &Address,
    knots: &'a mut KnotSet,
) -> Result<&'a mut Stitch, InklingError> {
    let knot_name = target.get_knot()?;
    let stitch_name = target.get_stitch()?;

    knots
        .get_mut(knot_name)
        .and_then(|knot| knot.stitches.get_mut(stitch_name))
        .ok_or(
            StackError::BadAddress {
                address: target.clone(),
            }
            .into(),
        )
}
