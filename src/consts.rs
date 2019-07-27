//! Constant markers used when parsing `Ink` lines.

/************************
 * Line content markers *
 ************************/

/// Marker for a non-sticky choice, which can only ever be followed once in the story.
pub const CHOICE_MARKER: char = '*';

/// Marker for a sticky choice, which can be followed multiple times in the story.
pub const STICKY_CHOICE_MARKER: char = '+';

/// Marker for a gather point.
pub const GATHER_MARKER: char = '-';

/// Marker for a divert to another knot, stitch or label in the story.
pub const DIVERT_MARKER: &'static str = "->";

/// Marker for glue which joins separate lines together without a newline character.
pub const GLUE_MARKER: &'static str = "<>";

/// Marker for a tag associated with a line in the story.
///
/// Multiple markers can be used  in a single line. All text content between markers
/// (or the end of the line) will be a single tag.
pub const TAG_MARKER: char = '#';

/********************
 * Sequence markers *
 ********************/

/// Marker for a cycle alternative sequence.
pub const CYCLE_MARKER: char = '&';

/// Marker for a once-only alternative sequence.
pub const ONCE_ONLY_MARKER: char = '!';

/// Marker for a shuffle alternative sequence.
pub const SHUFFLE_MARKER: char = '~';

/// Marker for sequence item separator.
pub const SEQUENCE_SEPARATOR: &'static str = "|";

/****************
 * Knot markers *
 ****************/

/// Marker for a knot, the main divisor of story content.
pub const KNOT_MARKER: &'static str = "==";

/// Marker for a stitch belonging to a knot.
pub const STITCH_MARKER: &'static str = "=";

/************************
 * Comment line markers *
 ************************/

/// Marker for line comments, which will be ignored when parsing a story.
pub const LINE_COMMENT_MARKER: &'static str = "//";

/// Marker for line comments which will print a reminder message when encountered.
pub const TODO_COMMENT_MARKER: &'static str = "TODO:";

/*****************************
 * Default names for objects *
 *****************************/

/// Default name for the root `Stitch` in a `Knot` and `Knot` in a `Story`.
///
/// Contains characters which are not allowed in addresses, so there can never be a conflict
/// with a name in the `Ink` story.
pub const ROOT_KNOT_NAME: &'static str = "$ROOT$";

/// Name of knot that marks that a branch of story content is done.
pub const DONE_KNOT: &'static str = "DONE";

/// Name of knot that marks that the story is finished.
pub const END_KNOT: &'static str = "END";
