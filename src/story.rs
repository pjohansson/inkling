use crate::knot::Knot;

use std::collections::HashMap;

#[derive(Debug)]
pub struct Story {
    knots: HashMap<String, Knot>,
    stack: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    
}