//! Content that alternates from a fixed set when processed.

use crate::{
    error::{parse::validate::ValidationError, utils::MetaData},
    follow::FollowData,
    knot::Address,
    line::LineChunk,
    story::validate::{ValidateContent, ValidationData},
};

#[cfg(feature = "random")]
use rand::seq::SliceRandom;

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
    /// Active list of item indices that will be used to select items.
    ///
    /// The list should be in reverse item order, so that we can pop indices from
    /// it -- popping yields the last item, after all.
    pub active_inds: Vec<usize>,
    /// Which kind of alternative this represents.
    pub kind: AlternativeKind,
    /// Set of content which the object will select and process from.
    pub items: Vec<LineChunk>,
}

impl Alternative {
    #[allow(unused_variables)] // `data` only used when the `random` feature is enabled
    /// Get the next item index in the alternative sequence.
    pub fn get_next_index(&mut self, data: &mut FollowData) -> Option<usize> {
        match self.kind {
            AlternativeKind::OnceOnly => self.active_inds.pop(),
            AlternativeKind::Sequence if self.active_inds.len() > 1 => self.active_inds.pop(),
            AlternativeKind::Sequence => self.active_inds.get(0).cloned(),
            AlternativeKind::Cycle => {
                if self.active_inds.is_empty() {
                    self.reset_active_list()
                }

                self.active_inds.pop()
            }
            AlternativeKind::Shuffle => {
                if self.active_inds.is_empty() {
                    self.reset_active_list()
                }

                #[cfg(feature = "random")]
                if self.is_first_item() {
                    self.active_inds.shuffle(&mut data.rng.gen);
                }

                self.active_inds.pop()
            }
        }
    }

    #[allow(dead_code)]
    /// Return whether or not we are at the first item in the sequence.
    fn is_first_item(&self) -> bool {
        self.active_inds.len() == self.items.len()
    }

    /// Reset the active list by remaking the index list and reversing it.
    fn reset_active_list(&mut self) {
        self.active_inds = (0..self.items.len()).rev().collect();
    }
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
    /// Shuffles the active list, then goes through it and cycles with a new shuffle at the end.
    ///
    /// # Example
    /// A set of three cards `[One, Two, Three]` will be shuffled then dealt one by one. Once
    /// the set is empty, the deck is reshuffled.
    Shuffle,
}

impl ValidateContent for Alternative {
    fn validate(
        &mut self,
        error: &mut ValidationError,
        current_location: &Address,
        meta_data: &MetaData,
        data: &ValidationData,
    ) {
        self.items
            .iter_mut()
            .for_each(|item| item.validate(error, current_location, meta_data, data));
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
            active_inds: (0..self.items.len()).rev().collect(),
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
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        follow::FollowDataBuilder, line::LineChunkBuilder,
        process::line::tests::mock_data_with_single_stitch, story::rng::StoryRng,
    };

    fn mock_data_with_single_stitch_and_rng(
        knot: &str,
        stitch: &str,
        num_visited: u32,
        rng: StoryRng,
    ) -> FollowData {
        use std::collections::HashMap;

        let mut stitch_count = HashMap::new();
        stitch_count.insert(stitch.to_string(), num_visited);

        let mut knot_visit_counts = HashMap::new();
        knot_visit_counts.insert(knot.to_string(), stitch_count);

        FollowDataBuilder::new()
            .with_knots(knot_visit_counts)
            .with_rng(rng)
            .build()
    }

    /// Create an `Alternative` of given kind with a certain number of items.
    fn create_alternative(kind: AlternativeKind, num_items: usize) -> Alternative {
        let mut builder = AlternativeBuilder::from_kind(kind);

        for i in 0..num_items {
            builder.add_line(LineChunkBuilder::from_string(&format!("Line {}", i + 1)).build());
        }

        builder.build()
    }

    #[test]
    fn alternative_builder_sets_active_list_as_reversed_indices_when_calling_build() {
        let items = vec![
            LineChunkBuilder::from_string("Line 1").build(),
            LineChunkBuilder::from_string("Line 2").build(),
            LineChunkBuilder::from_string("Line 3").build(),
            LineChunkBuilder::from_string("Line 4").build(),
        ];

        let builder = AlternativeBuilder {
            kind: AlternativeKind::Cycle,
            items: items.clone(),
        };

        let alternative = builder.build();

        assert_eq!(alternative.items, items);
        assert_eq!(&alternative.active_inds, &[3, 2, 1, 0]);
    }

    #[test]
    fn alternative_get_next_index_for_cycle_resets_list_after_yielding_all_inds() {
        let mut alternative = create_alternative(AlternativeKind::Cycle, 3);
        let mut data = mock_data_with_single_stitch("", "", 0);

        assert_eq!(alternative.get_next_index(&mut data), Some(0));
        assert_eq!(alternative.get_next_index(&mut data), Some(1));
        assert_eq!(alternative.get_next_index(&mut data), Some(2));
        assert_eq!(alternative.get_next_index(&mut data), Some(0));
        assert_eq!(alternative.get_next_index(&mut data), Some(1));
        assert_eq!(alternative.get_next_index(&mut data), Some(2));
        assert_eq!(alternative.get_next_index(&mut data), Some(0));
    }

    #[test]
    fn alternative_get_next_index_for_sequence_yields_final_index_forever_after_the_initial() {
        let mut alternative = create_alternative(AlternativeKind::Sequence, 3);
        let mut data = mock_data_with_single_stitch("", "", 0);

        assert_eq!(alternative.get_next_index(&mut data), Some(0));
        assert_eq!(alternative.get_next_index(&mut data), Some(1));
        assert_eq!(alternative.get_next_index(&mut data), Some(2));
        assert_eq!(alternative.get_next_index(&mut data), Some(2));
        assert_eq!(alternative.get_next_index(&mut data), Some(2));
    }

    #[test]
    fn alternative_get_next_index_for_once_only_yields_none_after_the_initial() {
        let mut alternative = create_alternative(AlternativeKind::OnceOnly, 3);
        let mut data = mock_data_with_single_stitch("", "", 0);

        assert_eq!(alternative.get_next_index(&mut data), Some(0));
        assert_eq!(alternative.get_next_index(&mut data), Some(1));
        assert_eq!(alternative.get_next_index(&mut data), Some(2));
        assert_eq!(alternative.get_next_index(&mut data), None);
        assert_eq!(alternative.get_next_index(&mut data), None);
    }

    #[cfg(not(feature = "random"))]
    /// Module for testing the behaviour of shuffle sequences without randomness.
    mod not_shuffle {
        use super::*;

        #[test]
        fn alternative_get_next_index_for_shuffle_resets_list_after_yielding_all_inds_if_not_random(
        ) {
            let mut alternative = create_alternative(AlternativeKind::Shuffle, 3);
            let mut data = mock_data_with_single_stitch_and_rng("", "", 0, StoryRng::default());

            assert_eq!(alternative.get_next_index(&mut data), Some(0));
            assert_eq!(alternative.get_next_index(&mut data), Some(1));
            assert_eq!(alternative.get_next_index(&mut data), Some(2));
            assert_eq!(alternative.get_next_index(&mut data), Some(0));
            assert_eq!(alternative.get_next_index(&mut data), Some(1));
            assert_eq!(alternative.get_next_index(&mut data), Some(2));
            assert_eq!(alternative.get_next_index(&mut data), Some(0));
        }
    }

    #[cfg(feature = "random")]
    /// Module for testing the behaviour of shuffle sequences when `random` is activated.
    mod shuffle {
        use super::*;

        // With 10 items, the probability of drawing a particular sequence is 1 / 10! = 2.75573-07
        const NUM_ITEMS: usize = 10;

        #[test]
        fn alternative_get_next_index_for_shuffle_shuffles_active_index_list() {
            let mut alternative = create_alternative(AlternativeKind::Shuffle, NUM_ITEMS);
            let mut data = mock_data_with_single_stitch_and_rng("", "", 0, StoryRng::default());

            // Create reverse list from 1, since we will pop the first (0) before the comparison
            let inds_unshuffled = (0..NUM_ITEMS).skip(1).rev().collect::<Vec<usize>>();

            alternative.get_next_index(&mut data);
            assert!(alternative.active_inds != inds_unshuffled);
        }

        #[test]
        fn alternative_get_next_index_for_shuffle_uses_shuffle_in_place_with_the_generator() {
            let mut alternative = create_alternative(AlternativeKind::Shuffle, NUM_ITEMS);

            let mut rng = StoryRng::default();
            let mut data = mock_data_with_single_stitch_and_rng("", "", 0, rng.clone());

            let mut active_inds = alternative.active_inds.clone();
            active_inds.shuffle(&mut rng.gen);

            assert_eq!(alternative.get_next_index(&mut data), active_inds.pop());
            assert_eq!(&alternative.active_inds, &active_inds);
        }

        #[test]
        fn alternative_get_next_index_for_shuffle_resets_list_after_emptying() {
            let mut alternative = create_alternative(AlternativeKind::Shuffle, NUM_ITEMS);

            let mut rng = StoryRng::default();
            let mut data = mock_data_with_single_stitch_and_rng("", "", 0, rng.clone());

            // Unshuffled list
            let mut active_inds = alternative.active_inds.clone();

            // First (internal) shuffle, go through all items
            for _ in 0..NUM_ITEMS {
                alternative.get_next_index(&mut data);
            }

            // Second shuffle will now occur, make corresponding shuffle for comparison
            rng.gen.set_word_pos(data.rng.gen.get_word_pos());
            active_inds.shuffle(&mut rng.gen);

            assert_eq!(alternative.get_next_index(&mut data), active_inds.pop());
            assert_eq!(&alternative.active_inds, &active_inds);
        }
    }
}
