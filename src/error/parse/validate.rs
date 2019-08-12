use crate::error::{
    parse::address::InvalidAddressError,
    utils::{write_line_information, MetaData},
};

use std::{
    error::Error,
    fmt::{self, Write},
};

impl Error for ValidationError {}

#[derive(Debug)]
pub struct ValidationError {
    invalid_address_errors: Vec<InvalidAddressError>,
    name_space_errors: Vec<NameSpaceCollision>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!();
    }
}

#[derive(Debug)]
pub struct NameSpaceCollision {
    pub from_name: String,
    pub from_kind: CollisionKind,
    pub from_meta_data: MetaData,
    pub to_name: String,
    pub to_kind: CollisionKind,
    pub to_meta_data: MetaData,
}

#[derive(Clone, Copy, Debug)]
pub enum CollisionKind {
    Knot,
    Stitch,
    Variable,
}

impl fmt::Display for NameSpaceCollision {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write_line_information(f, &self.from_meta_data)?;

        write!(
            f,
            "namespace collision between {} '{}' and {} '{}' (defined at {})",
            self.from_kind, self.from_name, self.to_kind, self.to_name, self.to_meta_data
        )
    }
}

impl fmt::Display for CollisionKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            CollisionKind::Knot => write!(f, "knot"),
            CollisionKind::Stitch => write!(f, "stitch"),
            CollisionKind::Variable => write!(f, "global variable"),
        }
    }
}

pub(crate) fn print_invalid_address_errors(
    errors: &[InvalidAddressError],
) -> Result<String, fmt::Error> {
    let mut buffer = String::new();

    for err in errors {
        write!(&mut buffer, "{}\n", err)?;
    }

    Ok(buffer)
}
