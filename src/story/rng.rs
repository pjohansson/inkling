//! Wrapper around a random number generator.
//!
//! The wrapper is needed because we only have a generator if the `random`
//! feature is enabled. If it is not enabled, we do not have the generator. Thus,
//! we wrap the generator (if needed) in the `StoryRng` struct, which is empty
//! if `random` is not enabled.
//!
//! This means that regardless of whether or not we need the generator, we have
//! and object that we can pass through the system and won't have to make a lot
//! of conditionals in the rest of the code. Only when the generator will be needed,
//! such as for generating the alternative shuffle sequences.

// For scope simplicity we create a private modules depending on whether or not
// the random generator will be needed. We then export the `StoryRng` object
// from the module.
pub use feature_wrapper::StoryRng;

#[cfg(not(feature = "random"))]
mod feature_wrapper {
    #[cfg(feature = "serde_support")]
    use serde::{Deserialize, Serialize};

    #[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
    #[cfg_attr(test, derive(PartialEq))]
    #[derive(Clone, Debug, Default)]
    /// Random number generator for the [`Story`][crate::story::Story].
    ///
    /// If the `random` is not enabled this is a dummy struct which will
    /// not be used when moving through the story. Otherwise, it holds the generator
    /// and complementary information for generating numbers.
    ///
    /// If you are reading this text, the `random` feature is **not**
    /// currently enabled.
    pub struct StoryRng;
}

#[cfg(feature = "random")]
mod feature_wrapper {
    use rand::{RngCore, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    #[cfg(feature = "serde_support")]
    use serde::{
        de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor},
        ser::{Serialize, SerializeStruct, Serializer},
    };

    #[derive(Clone, Debug)]
    /// Random number generator for the [`Story`][crate::story::Story].
    ///
    /// We use `ChaChaRng` due to it being seedable and with the ability to get and set
    /// the word position. This is necessary to restore the state when de/serializing.
    ///
    /// If the `serde_support` feature is enabled, we manually derived `Deserialize`
    /// and `Serialize` below. This is due to the generator itself not having either
    /// derived.
    pub struct StoryRng {
        /// Random number generator.
        pub gen: ChaCha8Rng,
        /// Seed for the generator.
        seed: u64,
    }

    impl Default for StoryRng {
        fn default() -> Self {
            let seed = ChaCha8Rng::from_entropy().next_u64();
            StoryRng::with_seed(seed)
        }
    }

    impl StoryRng {
        /// Initiate the random number generator with a seed.
        fn with_seed(seed: u64) -> Self {
            let mut gen = ChaCha8Rng::seed_from_u64(seed);

            // `get_word_pos()` will panic unless we set the stream to 0
            gen.set_word_pos(0);

            StoryRng { gen, seed }
        }

        #[cfg(feature = "serde_support")]
        /// Initiate the random number generator with a seed and word position.
        fn with_seed_and_position(seed: u64, position: u128) -> Self {
            let mut rng = Self::with_seed(seed);
            rng.gen.set_word_pos(position);

            rng
        }
    }

    #[cfg(feature = "serde_support")]
    impl Serialize for StoryRng {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            // The word position for `ChaCha8Rng` is u128 but `serde` can only serialize
            // up to u64. Ensure that we de/serialize as that.
            let position = self.gen.get_word_pos() as u64;

            let mut state = serializer.serialize_struct("StoryRng", 2)?;
            state.skip_field("gen")?;
            state.serialize_field("seed", &self.seed)?;
            state.serialize_field("position", &position)?;
            state.end()
        }
    }

    #[cfg(feature = "serde_support")]
    impl<'de> Deserialize<'de> for StoryRng {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            use std::fmt;

            enum Field {
                Seed,
                Position,
            };

            impl<'de> Deserialize<'de> for Field {
                fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    struct FieldVisitor;

                    impl<'de> Visitor<'de> for FieldVisitor {
                        type Value = Field;

                        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                            formatter.write_str("`seed` or `position`")
                        }

                        fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where
                            E: de::Error,
                        {
                            match value {
                                "seed" => Ok(Field::Seed),
                                "position" => Ok(Field::Position),
                                _ => Err(de::Error::unknown_field(value, FIELDS)),
                            }
                        }
                    }

                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct StoryRngVisitor;

            impl<'de> Visitor<'de> for StoryRngVisitor {
                type Value = StoryRng;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("struct StoryRng")
                }

                fn visit_seq<V>(self, mut seq: V) -> Result<StoryRng, V::Error>
                where
                    V: SeqAccess<'de>,
                {
                    let seed = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                    // The word position for `ChaCha8Rng` is u128 but `serde` can only serialize
                    // up to u64. Ensure that we de/serialize as that.
                    let position: u64 = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                    Ok(StoryRng::with_seed_and_position(seed, position as u128))
                }

                fn visit_map<V>(self, mut map: V) -> Result<StoryRng, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut seed = None;
                    let mut position = None;

                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Seed => {
                                if seed.is_some() {
                                    return Err(de::Error::duplicate_field("seed"));
                                }
                                seed = Some(map.next_value()?);
                            }
                            Field::Position => {
                                if position.is_some() {
                                    return Err(de::Error::duplicate_field("position"));
                                }
                                position = Some(map.next_value()?);
                            }
                        }
                    }

                    let seed = seed.ok_or_else(|| de::Error::missing_field("seed"))?;

                    // The word position for `ChaCha8Rng` is u128 but `serde` can only serialize
                    // up to u64. Ensure that we de/serialize as that.
                    let position: u64 =
                        position.ok_or_else(|| de::Error::missing_field("position"))?;

                    Ok(StoryRng::with_seed_and_position(seed, position as u128))
                }
            }

            const FIELDS: &'static [&'static str] = &["seed", "position"];
            deserializer.deserialize_struct("StoryRng", FIELDS, StoryRngVisitor)
        }
    }

    #[cfg(test)]
    // Implementation for `PartialEq` to satisfy bounds on `serde_test` functions
    impl PartialEq for StoryRng {
        fn eq(&self, other: &Self) -> bool {
            self.seed == other.seed && self.gen.get_word_pos() == other.gen.get_word_pos()
        }
    }

    #[cfg(all(test, feature = "serde_support"))]
    mod tests {
        use super::*;
        use serde_test::*;

        #[test]
        fn story_rng_serializes_with_seed() {
            let seed = 30;
            let rng = StoryRng::with_seed(seed);

            let position = rng.gen.get_word_pos() as u64;

            assert_tokens(
                &rng,
                &[
                    Token::Struct {
                        name: "StoryRng",
                        len: 2,
                    },
                    Token::Str("seed"),
                    Token::U64(seed),
                    Token::Str("position"),
                    Token::U64(position),
                    Token::StructEnd,
                ],
            );
        }

        #[test]
        fn story_rng_serializes_with_correct_word_position() {
            let seed = 30;
            let mut rng = StoryRng::with_seed(seed);

            let mut buffer = vec![0; 64];
            rng.gen.fill_bytes(&mut buffer);

            // Get and sanity check current position, it should not be zero right now
            let position = rng.gen.get_word_pos() as u64;
            assert!(position > 0);

            assert_tokens(
                &rng,
                &[
                    Token::Struct {
                        name: "StoryRng",
                        len: 2,
                    },
                    Token::Str("seed"),
                    Token::U64(seed),
                    Token::Str("position"),
                    Token::U64(position),
                    Token::StructEnd,
                ],
            );
        }
    }
}
