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
    pub invalid_address_errors: Vec<InvalidAddressError>,
    pub name_space_errors: Vec<NameSpaceCollision>,
}

impl ValidationError {
    pub fn is_empty(&self) -> bool {
        self.invalid_address_errors.is_empty() && self.name_space_errors.is_empty()
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!();
    }
}

#[derive(Debug)]
pub struct NameSpaceCollision {
    pub name: String,
    pub from_kind: CollisionKind,
    pub from_meta_data: MetaData,
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
            "namespace collision between {} of name '{}' and a {} previously defined at {}",
            self.from_kind, self.name, self.to_kind, self.to_meta_data
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
