//! Content that alternates from a fixed set when processed.

use crate::{
    error::{parse::address::InvalidAddressError, utils::MetaData},
    knot::{Address, ValidateAddressData, ValidateAddresses},
    line::LineChunk,
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Set of line content which can vary when it is processed.
///
/// The variational content comes from a fixed set of chunks. When the `Alternative`
/// is processed it will pick one item from this set and process it. Which item is
/// selected depends on which kind of alternative it is.
///
/// Any selected `LineChunk`s can of course contain nested alternatives, and so on.
pub struct Alternative {
    /// Current index in the set of content.
    pub current_index: Option<usize>,
    /// Which kind of alternative this represents.
    pub kind: AlternativeKind,
    /// Set of content which the object will select and process from.
    pub items: Vec<LineChunk>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Variants of alternating content.
pub enum AlternativeKind {
    /// Cycles through the set, starting from the beginning after reaching the end.
    ///
    /// # Example
    /// A set of the week days `[Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday]`
    /// will in turn print every day, then start over again from Monday after Sunday has been
    /// visited.
    Cycle,
    /// Goes through the set of content once, then produces nothing.
    ///
    /// # Example
    /// A countdown from `[Three, Two, One]` will print the numbers, then nothing after
    /// the last item has been shown.
    OnceOnly,
    /// Goes through the set of content once, then repeats the final item.
    ///
    /// # Example
    /// A train traveling to its destination `[Frankfurt, Mannheim, Heidelberg]` will print
    /// each destination, then `Heidelberg` forever after reaching the city.
    Sequence,
}

impl ValidateAddresses for Alternative {
    fn validate(
        &mut self,
        errors: &mut Vec<InvalidAddressError>,
        meta_data: &MetaData,
        current_address: &Address,
        data: &ValidateAddressData,
    ) {
        self.items
            .iter_mut()
            .for_each(|item| item.validate(errors, meta_data, current_address, data));
    }

    #[cfg(test)]
    fn all_addresses_are_valid(&self) -> bool {
        self.items.iter().all(|item| item.all_addresses_are_valid())
    }
}

/// Builder struct for `Alternative`.
pub struct AlternativeBuilder {
    kind: AlternativeKind,
    items: Vec<LineChunk>,
}

impl AlternativeBuilder {
    /// Construct the builder with the given `AlternativeKind`.
    pub fn from_kind(kind: AlternativeKind) -> Self {
        AlternativeBuilder {
            kind,
            items: Vec::new(),
        }
    }

    /// Finalize the `Alternative` and return it.
    pub fn build(self) -> Alternative {
        Alternative {
            current_index: None,
            kind: self.kind,
            items: self.items,
        }
    }

    /// Set the alternative `LineChunk`s to the builder.
    ///
    /// # Notes
    /// *   Replaces the current set of items.
    pub fn with_items(mut self, items: Vec<LineChunk>) -> Self {
        self.items = items;
        self
    }

    #[cfg(test)]
    /// Construct a builder with `AlternativeKind::Cycle`.
    pub fn cycle() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::Cycle)
    }

    #[cfg(test)]
    /// Construct a builder with `AlternativeKind::OnceOnly`.
    pub fn once_only() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::OnceOnly)
    }

    #[cfg(test)]
    /// Construct a builder with `AlternativeKind::Sequence`.
    pub fn sequence() -> Self {
        AlternativeBuilder::from_kind(AlternativeKind::Sequence)
    }

    #[cfg(test)]
    /// Add a chunk of line content to the set of alternatives.
    pub fn add_line(&mut self, line: LineChunk) {
        self.items.push(line);
    }

    #[cfg(test)]
    /// Add a chunk of line content to the set of alternatives.
    pub fn with_line(mut self, line: LineChunk) -> Self {
        self.add_line(line);
        self
    }
}
