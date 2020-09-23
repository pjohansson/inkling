//! Structures representing knots and stitches in a story.
//!
//! Knots are the main divisions of content in a story, with stitches belonging
//! to knots as a secondary structuring. These are the two levels of story content:
//! further subdivision is not implemented in the `Ink` language.
//!
//! Lines of text content is organized in these knots and stitches. In `inkling`
//! we keep all this text content in `Stitch`es, which belong to parent `Knot`s.
//! When a knot is encountered it will point the story flow into a default stitch
//! from which the text content will be parsed.
//!
//! Content in unnamed stitches or pure knots (any lines before a stitch marker
//! is encountered in an `Ink` story file) will be placed in a stitch with a default
//! name. This name will not overlap with the allowed namespace of knots or stitches,
//! so there can be no collisions.

use crate::{
    consts::{KNOT_MARKER, RESERVED_KEYWORDS, STITCH_MARKER},
    error::{
        parse::knot::{KnotErrorKind, KnotNameError},
        utils::MetaData,
    },
    follow::{EncounteredEvent, FollowData, FollowResult, LineDataBuffer},
    line::parse_line,
    node::{parse_root_node, Follow, RootNode, Stack},
};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// Convenience type for a set of `Knot`s.
///
/// The knot names are used as keys in the collection.
pub type KnotSet = HashMap<String, Knot>;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Knots groups story content into bits. Knots are further subdivided into `Stitch`es,
/// which contain the content.
///
/// Content in `Stitch`es is accessed through the contained hash map which is indexed
/// by their names. Knot content that belongs to the knot itself and not grouped under
/// a named stitch is placed in the map with a [default key][crate::consts::ROOT_KNOT_NAME] .
pub struct Knot {
    /// Name of `Stitch` that is used when diverting to the `Knot` without specifying
    /// a `Stitch`.
    pub default_stitch: String,
    /// Map of `Stitches` belonging to this `Knot`.
    pub stitches: HashMap<String, Stitch>,
    /// Tags associated with this knot.
    pub tags: Vec<String>,
    /// Information about the origin of this knot in the story file or text.
    pub meta_data: MetaData,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "serde_support", derive(Deserialize, Serialize))]
/// Stitches contain the actual story content and are grouped in larger `Knot`s.
pub struct Stitch {
    /// Graph of story content, which may or may not branch.
    pub root: RootNode,
    /// Last recorded position inside the `root` graph of content.
    pub stack: Stack,
    /// Information about the origin of this stitch in the story file or text.
    pub meta_data: MetaData,
}

impl Stitch {
    /// Follow a story while reading every line into a buffer.
    pub fn follow(&mut self, buffer: &mut LineDataBuffer, data: &mut FollowData) -> FollowResult {
        let result = self.root.follow(&mut self.stack, buffer, data)?;

        match &result {
            EncounteredEvent::Done | EncounteredEvent::Divert(..) => self.reset_stack(),
            EncounteredEvent::BranchingChoice(..) => (),
        }

        Ok(result)
    }

    /// Follow a story while reading every line into a buffer.
    pub fn follow_with_choice(
        &mut self,
        choice_index: usize,
        buffer: &mut LineDataBuffer,
        data: &mut FollowData,
    ) -> FollowResult {
        let result =
            self.root
                .follow_with_choice(choice_index, 0, &mut self.stack, buffer, data)?;

        match result {
            EncounteredEvent::Done | EncounteredEvent::Divert(..) => self.reset_stack(),
            _ => (),
        }

        Ok(result)
    }

    /// Reset the current stack to the first line of the root node.
    fn reset_stack(&mut self) {
        self.stack = vec![0];
    }
}

/// Parse a set of input lines into a `Stitch`.
pub fn parse_stitch_from_lines(
    lines: &[(&str, MetaData)],
    knot: &str,
    stitch: &str,
    meta_data: MetaData,
) -> Result<Stitch, Vec<KnotErrorKind>> {
    if lines.is_empty() {
        return Err(vec![KnotErrorKind::EmptyStitch {
            name: Some(stitch.to_string()),
            meta_data: meta_data.clone(),
        }]);
    }

    let mut parsed_lines = Vec::new();
    let mut line_errors = Vec::new();

    for result in lines
        .into_iter()
        .map(|(line, meta_data)| parse_line(line, meta_data))
    {
        match result {
            Ok(parsed_line) => parsed_lines.push(parsed_line),
            Err(line_error) => line_errors.push(KnotErrorKind::from(line_error)),
        }
    }

    if line_errors.is_empty() {
        let root = parse_root_node(&parsed_lines, knot, stitch);

        Ok(Stitch {
            root,
            stack: vec![0],
            meta_data,
        })
    } else {
        Err(line_errors)
    }
}

/// Read a knot name from a non-parsed string which contains text markers for a knot.
///
/// The name is validated before returning.
pub fn read_knot_name(line: &str) -> Result<String, KnotNameError> {
    if line.trim_start().starts_with(KNOT_MARKER) {
        read_name_with_marker(line)
    } else {
        Err(KnotNameError::Empty)
    }
}

/// Read a stitch name from a non-parsed string which contains text markers for a stitch.
///
/// The name is validated before returning.
pub fn read_stitch_name(line: &str) -> Result<String, KnotNameError> {
    if line.trim_start().starts_with(STITCH_MARKER) && !line.trim_start().starts_with(KNOT_MARKER) {
        read_name_with_marker(line)
    } else {
        Err(KnotNameError::Empty)
    }
}

/// Read a name beginning with the given knot or stitch marker.
///
/// The name is validated before returning.
///
/// # Notes
///  *  Uses the [stitch marker][crate::consts::STITCH_MARKER] to trim extraneous markers
///     from the line before validating the name. Since the stitch marker is a subset
///     of the knot marker this will trim both types, but any other marker will not be
///     trimmed from the line.
fn read_name_with_marker(line: &str) -> Result<String, KnotNameError> {
    let trimmed_name = line
        .trim_start_matches(STITCH_MARKER)
        .trim_end_matches(STITCH_MARKER)
        .trim();

    if let Some(c) = trimmed_name
        .chars()
        .find(|&c| !(c.is_alphanumeric() || c == '_'))
    {
        if c.is_whitespace() {
            Err(KnotNameError::ContainsWhitespace)
        } else {
            Err(KnotNameError::ContainsInvalidCharacter(c))
        }
    } else if trimmed_name.is_empty() {
        Err(KnotNameError::Empty)
    } else if RESERVED_KEYWORDS.contains(&trimmed_name.to_uppercase().as_str()) {
        Err(KnotNameError::ReservedKeyword {
            keyword: trimmed_name.to_string(),
        })
    } else {
        Ok(trimmed_name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        error::parse::line::LineError,
        follow::FollowDataBuilder,
        knot::{get_num_visited, Address},
        line::{InternalLine, ParsedLineKind},
    };

    use std::str::FromStr;

    impl FromStr for Stitch {
        type Err = ();

        fn from_str(content: &str) -> Result<Self, Self::Err> {
            let lines = parse_lines(content).unwrap();
            let root = parse_root_node(&lines, "", "");

            Ok(Stitch {
                root,
                stack: vec![0],
                meta_data: MetaData { line_index: 0 },
            })
        }
    }

    fn parse_lines(s: &str) -> Result<Vec<ParsedLineKind>, LineError> {
        s.lines().map(|line| parse_line(line, &().into())).collect()
    }

    fn mock_follow_data(stitch: &Stitch) -> FollowData {
        let (knot, stitch) = stitch.root.address.get_knot_and_stitch().unwrap();

        let mut stitch_count = HashMap::new();
        stitch_count.insert(stitch.to_string(), 0);

        let mut knot_visit_counts = HashMap::new();
        knot_visit_counts.insert(knot.to_string(), stitch_count);

        FollowDataBuilder::new()
            .with_knots(knot_visit_counts)
            .build()
    }

    #[test]
    fn parsing_stitch_sets_root_node_address() {
        let stitch =
            parse_stitch_from_lines(&[("", ().into())], "tripoli", "cinema", ().into()).unwrap();

        assert_eq!(
            stitch.root.address,
            Address::from_parts_unchecked("tripoli", Some("cinema")),
        );
    }

    #[test]
    fn parsing_empty_stitch_yields_error() {
        assert!(parse_stitch_from_lines(&[], "tripoli", "cinema", ().into()).is_err());
    }

    #[test]
    fn parsing_stitch_sets_meta_data_from_given() {
        let stitch =
            parse_stitch_from_lines(&[("", ().into())], "tripoli", "cinema", MetaData::from(10))
                .unwrap();

        assert_eq!(stitch.meta_data.line_index, 10);
    }

    #[test]
    fn stitch_restarts_from_their_first_line_when_run_again() {
        let text = "Hello, World!";

        let mut stitch = Stitch::from_str(text).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&stitch);

        stitch.follow(&mut buffer, &mut data).unwrap();
        stitch.follow(&mut buffer, &mut data).unwrap();

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, text);
        assert_eq!(&buffer[1].text, text);
    }

    #[test]
    fn following_stitch_increases_the_number_of_visits() {
        let text = "Hello, World!";

        let mut stitch = Stitch::from_str(text).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&stitch);

        stitch.follow(&mut buffer, &mut data).unwrap();
        stitch.follow(&mut buffer, &mut data).unwrap();

        assert_eq!(get_num_visited(&stitch.root.address, &data).unwrap(), 2);
    }

    #[test]
    fn following_stitch_with_choice_does_not_increase_the_number_of_visits() {
        let text = "*   Choice";

        let mut stitch = Stitch::from_str(text).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&stitch);

        stitch
            .follow_with_choice(0, &mut buffer, &mut data)
            .unwrap();

        assert_eq!(get_num_visited(&stitch.root.address, &data).unwrap(), 0);
    }

    #[test]
    fn after_resuming_follow_from_a_gather_point_the_number_of_visits_is_not_increased() {
        let text = "\
*   Choice 1
*   Choice 2
-   Line
";

        let mut stitch = Stitch::from_str(text).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&stitch);

        stitch
            .follow_with_choice(0, &mut buffer, &mut data)
            .unwrap();

        assert_eq!(buffer.last().unwrap().text.trim(), "Line");
        assert_eq!(get_num_visited(&stitch.root.address, &data).unwrap(), 0);
    }

    #[test]
    fn stitch_with_divert_shortcuts_at_it() {
        let name = "fool".to_string();

        let pre = "Mrs. Bennet was making a fool of herself.";
        let after = "After Mrs. Bennet left, Elizabet went upstairs to look after Jane.";

        let text = format!(
            "\
{}
-> {}
{}
",
            pre, name, after
        );

        let mut stitch = Stitch::from_str(&text).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&stitch);

        assert_eq!(
            stitch.follow(&mut buffer, &mut data).unwrap(),
            EncounteredEvent::Divert(Address::Raw(name))
        );

        assert_eq!(buffer.len(), 2);
        assert_eq!(&buffer[0].text, pre);
        assert_eq!(buffer[1].text.trim(), "");
    }

    #[test]
    fn stitch_with_choice_returns_it() {
        let choice1 = "Choice 1";
        let choice2 = "Choice 2";

        let lines = vec![
            "Hello, world!".to_string(),
            format!("* {}", choice1),
            format!("* {}", choice2),
        ];

        let mut text = String::new();
        for line in lines.iter() {
            text.push_str(&line);
            text.push('\n');
        }

        let mut stitch = Stitch::from_str(&text).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&stitch);

        let choices = match stitch.follow(&mut buffer, &mut data).unwrap() {
            EncounteredEvent::BranchingChoice(choices) => choices,
            _ => panic!("did not get a `BranchingChoice`"),
        };

        assert_eq!(choices.len(), 2);

        assert_eq!(
            choices[0].choice_data.display_text,
            InternalLine::from_string("Choice 1")
        );
        assert_eq!(
            choices[1].choice_data.display_text,
            InternalLine::from_string("Choice 2")
        );
    }

    #[test]
    fn following_choice_adds_choice_text_to_buffer() {
        let choice = "Choice 1";
        let text = format!("* {}", choice);

        let mut stitch = Stitch::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();
        let mut data = mock_follow_data(&stitch);

        stitch.follow(&mut buffer, &mut data).unwrap();
        stitch
            .follow_with_choice(0, &mut buffer, &mut data)
            .unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(&buffer[0].text, choice);
    }

    #[test]
    fn when_a_stitch_is_finished_the_stack_is_reset() {
        let text = "\
* Choice 1
* Choice 2
";

        let mut stitch = Stitch::from_str(text).unwrap();

        let mut buffer = Vec::new();
        let mut data = mock_follow_data(&stitch);

        stitch.follow(&mut buffer, &mut data).unwrap();
        assert_eq!(&stitch.stack, &[0]);

        stitch
            .follow_with_choice(0, &mut buffer, &mut data)
            .unwrap();
        assert_eq!(&stitch.stack, &[0]);
    }

    #[test]
    fn stitch_with_choice_follows_into_choice() {
        let line1 = "A Scandal in Bohemia";
        let line2 = "The Scarlet Letter";
        let line_unused = "Moby Dick; Or, the Whale";

        let lines = vec![
            format!("* Choice 1"),
            format!("{}", line_unused),
            format!("* Choice 2"),
            format!("{}", line1),
            format!("{}", line2),
        ];

        let mut text = String::new();
        for line in lines.iter() {
            text.push_str(&line);
            text.push('\n');
        }

        let mut stitch = Stitch::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();
        let mut data = mock_follow_data(&stitch);

        stitch.follow(&mut buffer, &mut data).unwrap();
        stitch
            .follow_with_choice(1, &mut buffer, &mut data)
            .unwrap();

        assert_eq!(buffer.len(), 3);
        assert_eq!(&buffer[1].text, line1);
        assert_eq!(&buffer[2].text, line2);
    }

    #[test]
    fn stitch_gathers_all_choices_at_requested_level() {
        let line1 = "The Thief";
        let line2 = "Sanshirō ";

        let lines = vec![
            format!("*  Choice 1"),
            format!("   The Scarlet Letter"),
            format!("   Moby Dick; Or, the Whale"),
            format!("*  Choice 2"),
            format!("   Den vedervärdige mannen från Säffle"),
            format!("- {}", line1),
            format!("{}", line2),
        ];

        let mut text = String::new();
        for line in lines.iter() {
            text.push_str(&line);
            text.push('\n');
        }

        let mut stitch = Stitch::from_str(&text).unwrap();
        let mut data = mock_follow_data(&stitch);

        let mut results_choice1 = LineDataBuffer::new();

        stitch.follow(&mut results_choice1, &mut data).expect("one");
        stitch
            .follow_with_choice(0, &mut results_choice1, &mut data)
            .expect("two");

        let mut results_choice2 = LineDataBuffer::new();

        stitch
            .follow(&mut results_choice2, &mut data)
            .expect("three");
        stitch
            .follow_with_choice(1, &mut results_choice2, &mut data)
            .expect("four");

        assert_eq!(results_choice1[3], results_choice2[2]);
        assert_eq!(results_choice1[4], results_choice2[3]);
    }

    #[test]
    fn stitch_can_follow_multiple_level_choices_and_gathers() {
        let text = "\
Line 1
*   Choice 1
    * *     Choice 1-1
    * *     Choice 1-2
            Line 2
    - -     Line 3
    * *     Choice 1-3
            Line 4
*   Choice 2
-   Line 5
Line 6
";
        let mut stitch = Stitch::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();
        let mut data = mock_follow_data(&stitch);

        stitch.follow(&mut buffer, &mut data).unwrap();
        stitch
            .follow_with_choice(0, &mut buffer, &mut data)
            .unwrap();
        stitch
            .follow_with_choice(1, &mut buffer, &mut data)
            .unwrap();
        stitch
            .follow_with_choice(0, &mut buffer, &mut data)
            .unwrap();

        // Four lines in choice, three choice lines and two lines after the gather
        assert_eq!(buffer.len(), 4 + 3 + 2);
    }

    #[test]
    fn follow_returns_error_if_out_of_bounds_index_is_followed_with() {
        let text = "\
*   Choice 1
*   Choice 2
";
        let mut stitch = Stitch::from_str(&text).unwrap();

        let mut buffer = LineDataBuffer::new();
        let mut data = mock_follow_data(&stitch);

        stitch.follow(&mut buffer, &mut data).unwrap();

        match stitch.follow_with_choice(2, &mut buffer, &mut data) {
            Err(_) => (),
            _ => panic!("expected a `InklingError::InvalidChoice` but did not get it"),
        }
    }

    #[test]
    fn read_knot_name_from_string_works_with_at_least_two_equal_signs() {
        assert_eq!(&read_knot_name("== Knot").unwrap(), "Knot");
        assert_eq!(&read_knot_name("=== Knot").unwrap(), "Knot");
        assert_eq!(&read_knot_name("== Knot==").unwrap(), "Knot");
        assert_eq!(&read_knot_name("==Knot==").unwrap(), "Knot");
    }

    #[test]
    fn read_stitch_name_from_string_works_with_exactly_one_equal_sign() {
        assert_eq!(&read_stitch_name("= Stitch").unwrap(), "Stitch");
        assert_eq!(&read_stitch_name("=Stitch").unwrap(), "Stitch");
        assert!(&read_stitch_name("== Stitch").is_err());
    }

    #[test]
    fn knot_name_must_be_single_word() {
        assert!(read_knot_name("== Knot name").is_err());
        assert!(read_knot_name("== Knot name ==").is_err());

        match read_knot_name("== knot name") {
            Err(KnotNameError::ContainsWhitespace) => (),
            Err(err) => panic!(
                "Expected a `KnotNameError::ContainsWhitespace` error, got {:?}",
                err
            ),
            _ => panic!("Invalid knot name did not raise error"),
        }
    }

    #[test]
    fn knot_name_cannot_be_empty() {
        assert!(read_knot_name("==").is_err());
        assert!(read_knot_name("== ").is_err());
        assert!(read_knot_name("== a").is_ok());

        match read_knot_name("== ") {
            Err(KnotNameError::Empty) => (),
            err => panic!(
                "expected `KnotNameError::Empty` as kind error, but got {:?}",
                err
            ),
        }
    }

    #[test]
    fn knot_name_can_only_contain_alphanumeric_characters_and_underlines() {
        assert!(read_knot_name("== knot").is_ok());
        assert!(read_knot_name("== knot_name").is_ok());
        assert!(read_knot_name("== knot_name_with_123").is_ok());
        assert!(read_knot_name("== knot_name_with_абв").is_ok());
        assert!(read_knot_name("== knot_name_with_αβγ").is_ok());
        assert!(read_knot_name("== knot_name_with_ñßüåäö").is_ok());
        assert!(read_knot_name("== knot_name_with_京").is_ok());

        assert!(read_knot_name("== knot.name").is_err());
        assert!(read_knot_name("== knot-name").is_err());
        assert!(read_knot_name("== knot/name").is_err());
        assert!(read_knot_name("== knot$name").is_err());

        match read_knot_name("== 京knot.name") {
            Err(KnotNameError::ContainsInvalidCharacter('.')) => (),
            Err(KnotNameError::ContainsInvalidCharacter(c)) => panic!(
                "Expected a `KnotNameError::ContainsInvalidCharacter` error \
                 with '.' as contained character, but got '{}'",
                c
            ),
            Err(err) => panic!(
                "Expected a `KnotNameError::ContainsInvalidCharacters` error, got {:?}",
                err
            ),
            _ => panic!("Invalid knot name did not raise error"),
        }
    }

    #[test]
    fn read_knot_name_from_string_returns_error_if_just_one_or_no_equal_signs() {
        assert!(read_knot_name("= Knot name ==").is_err());
        assert!(read_knot_name("=Knot name").is_err());
        assert!(read_knot_name(" Knot name ==").is_err());
        assert!(read_knot_name("Knot name==").is_err());
    }

    #[test]
    fn knot_and_stitch_names_may_not_be_from_the_reserved_list() {
        assert!(read_knot_name("== else").is_err());
        assert!(read_knot_name("== not").is_err());
    }
}
