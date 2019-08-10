use crate::error::parse::address::InvalidAddressError;

use std::fmt::{self, Write};

pub(crate) fn print_invalid_address_errors(errors: &[InvalidAddressError]) -> Result<String, fmt::Error> {
    let mut buffer = String::new();

    for err in errors {
        write!(&mut buffer, "{}\n", err)?;
    }

    Ok(buffer)
}
