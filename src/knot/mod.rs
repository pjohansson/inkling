//! Story structure collections: `Knot`s, `Stitch`es and utilities.

mod address;
mod stitch;
mod utils;

pub use address::{validate_addresses_in_knots, Address, ValidateAddresses};
pub use stitch::{
    parse_stitch_from_lines, read_knot_name, read_stitch_name, Knot, KnotSet, Stitch,
};
pub use utils::{
    get_empty_knot_counts, get_mut_stitch, get_num_visited, get_stitch, increment_num_visited,
};
