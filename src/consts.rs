//! Constant markers used when parsing `Ink` lines.

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

/// Marker for a knot, the main divisor of story content.
pub const KNOT_MARKER: &'static str = "==";

/// Marker for a stitch belonging to a knot.
pub const STITCH_MARKER: &'static str = "=";

/// Marker for line comments, which will be ignored when parsing a story.
pub const LINE_COMMENT_MARKER: &'static str = "//";

/// Marker for line comments which will print a reminder message when encountered.
pub const TODO_COMMENT_MARKER: &'static str = "TODO:";

/// Default name for the root `Stitch` in a `Knot` and `Knot` in a `Story`.
///
/// Contains characters which are not allowed in addresses, so there can never be a conflict
/// with a name in the `Ink` story.
pub const ROOT_KNOT_NAME: &'static str = "$ROOT$";

/// Name of knot that marks that a branch of story content is done.
pub const DONE_KNOT: &'static str = "DONE";

/// Name of knot that marks that the story is finished.
pub const END_KNOT: &'static str = "END";
