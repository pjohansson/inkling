//! Various utilities for accessing and handling story content.

use crate::{
    knot::Address,
    line::Variable,
    story::{
        story::Story,
        types::{LineBuffer, Location},
    },
};

/// Read all text from lines in a buffer into a single string and return it.
///
/// # Examples
/// ```
/// # use inkling::{read_story_from_string, utils::copy_lines_into_string};
/// let content = "\
/// Gamle gode Väinämöinen
/// rustade sig nu att resa
/// bort till kyligare trakter
/// till de dunkla Nordanlanden.
/// ";
///
/// let mut story = read_story_from_string(content).unwrap();
/// let mut line_buffer = Vec::new();
///
/// story.resume(&mut line_buffer);
///
/// let text = copy_lines_into_string(&line_buffer);
/// assert_eq!(&text, content);
/// ```
pub fn copy_lines_into_string(line_buffer: &LineBuffer) -> String {
    line_buffer
        .iter()
        .map(|line| line.text.clone())
        .collect::<Vec<_>>()
        .join("")
}

/// Create a divert [`Variable`][crate::line::Variable] to a specified location of a `Story`.
///
/// The location is validated against the locations in the story. If the location does
/// not exist, `None` is yielded.
///
/// # Example
/// ```
/// # use inkling::{read_story_from_string, utils::create_divert, Location};
/// let content = "\
/// == dream
/// = interior
/// GESICHT'S BEDROOM, MORNING
///
/// = wake
/// Gesicht is lying in his bed, eyes wide open and staring at the ceiling.
/// ";
///
/// let story = read_story_from_string(content).unwrap();
///
/// assert!(create_divert(&Location::from("dream"), &story).is_some());
/// assert!(create_divert(&Location::from("dream.wake"), &story).is_some());
/// ```
///
/// Diverts cannot be created to locations which are not in the story.
/// ```
/// # use inkling::{read_story_from_string, utils::create_divert, Location};
/// # let content = "\
/// # == dream
/// # = interior
/// # GESICHT'S BEDROOM, MORNING
/// # = wake
/// # Gesicht is lying in his bed, eyes wide open and staring at the ceiling.
/// # ";
/// # let story = read_story_from_string(content).unwrap();
/// assert!(create_divert(&Location::from("cornered"), &story).is_none());
/// assert!(create_divert(&Location::from("dream.breakfast"), &story).is_none());
/// ```
///
/// If not specified, the first stitch in the knot is used.
/// ```
/// # use inkling::{read_story_from_string, utils::create_divert, Location};
/// # let content = "\
/// # == dream
/// # = interior
/// # GESICHT'S BEDROOM, MORNING
/// # = wake
/// # Gesicht is lying in his bed, eyes wide open and staring at the ceiling.
/// # ";
/// # let story = read_story_from_string(content).unwrap();
/// assert_eq!(
///     create_divert(&Location::from("dream"), &story).unwrap(),
///     create_divert(&Location::from("dream.interior"), &story).unwrap()
/// );
/// ```
pub fn create_divert(to: &Location, story: &Story) -> Option<Variable> {
    Address::from_location(to, &story.knots)
        .ok()
        .map(|address| Variable::Divert(address))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        knot::Address,
        story::{story::read_story_from_string, types::Line},
    };

    #[test]
    fn string_from_line_buffer_joins_without_extra_newlines() {
        let lines = vec![
            Line {
                text: "Start of line, ".to_string(),
                tags: Vec::new(),
            },
            Line {
                text: "end of line without new lines".to_string(),
                tags: Vec::new(),
            },
        ];

        assert_eq!(
            &copy_lines_into_string(&lines),
            "Start of line, end of line without new lines"
        );
    }

    #[test]
    fn create_divert_variable_works_for_knots_in_story() {
        let content = "
            == knot_one
            Knot content.

            == knot_two
            Knot content.
        ";

        let story = read_story_from_string(content).unwrap();

        let divert_one = create_divert(&Location::from("knot_one"), &story);
        let target_one = Variable::Divert(Address::from_parts_unchecked("knot_one", None));
        assert_eq!(divert_one, Some(target_one));

        let divert_two = create_divert(&Location::from("knot_two"), &story);
        let target_two = Variable::Divert(Address::from_parts_unchecked("knot_two", None));
        assert_eq!(divert_two, Some(target_two));
    }

    #[test]
    fn create_divert_variable_works_for_stitches_in_story() {
        let content = "
            == knot_one
            Knot content.

            == knot_two
            Knot content.

            = stitch_one
            Stitch content.

            = stitch_two
            Stitch content.
        ";

        let story = read_story_from_string(content).unwrap();

        let divert = create_divert(&Location::from("knot_two.stitch_one"), &story);
        let target = Variable::Divert(Address::from_parts_unchecked(
            "knot_two",
            Some("stitch_one"),
        ));
        assert_eq!(divert, Some(target));
    }

    #[test]
    fn create_divert_variable_yields_none_if_knot_does_not_exist() {
        let content = "
            == knot_one
            Knot content.

            == knot_two
            Knot content.

            = stitch_one
            Stitch content.
        ";

        let story = read_story_from_string(content).unwrap();

        assert!(create_divert(&Location::from(""), &story).is_none());
        assert!(create_divert(&Location::from("knot_three"), &story).is_none());
        assert!(create_divert(&Location::from("knot_three.stitch_one"), &story).is_none());
    }

    #[test]
    fn create_divert_variable_yields_none_if_stitch_does_not_exist() {
        let content = "
            == knot_one
            Knot content.

            == knot_two
            Knot content.

            = stitch_one
            Stitch content.
        ";

        let story = read_story_from_string(content).unwrap();

        assert!(create_divert(&Location::from("knot_two.stitch_two"), &story).is_none());
        assert!(create_divert(&Location::from("knot_one.stitch_one"), &story).is_none());
    }

    #[test]
    fn create_divert_variable_defaults_to_root_stitch_if_not_specified() {
        let content = "
            == knot_one
            Knot content.

            == knot_two
            = stitch_one
            Stitch content.
        ";

        let story = read_story_from_string(content).unwrap();

        let divert_one = create_divert(&Location::from("knot_one"), &story);
        let target_one = Variable::Divert(Address::from_parts_unchecked("knot_one", None));
        assert_eq!(divert_one, Some(target_one));

        let divert_two = create_divert(&Location::from("knot_two"), &story);
        let target_two = Variable::Divert(Address::from_parts_unchecked(
            "knot_two",
            Some("stitch_one"),
        ));
        assert_eq!(divert_two, Some(target_two));
    }
}
