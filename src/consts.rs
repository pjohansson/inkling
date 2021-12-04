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

/// Marker for stopping item separator.
pub const STOPPING_SEPARATOR: &'static str = "|";

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

#[allow(dead_code)]
/// Marker to begin multiline comments.
pub const MULTILINE_COMMENT_BEGIN_MARKER: &'static str = "/*";

#[allow(dead_code)]
/// Marker to end multiline comments.
pub const MULTILINE_COMMENT_END_MARKER: &'static str = "*/";

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

/********************
 * Variable markers *
 ********************/

/// Marker for constant variable.
pub const CONST_MARKER: &'static str = "CONST";

/// Marker for global variable.
pub const VARIABLE_MARKER: &'static str = "VAR";

#[allow(dead_code)]
/// Marker for lists.
pub const LIST_MARKER: &'static str = "LIST";

#[allow(dead_code)]
/// Variable assignment marker.
pub const ASSIGNMENT_MARKER: char = '~';

/***********************
 * Meta data variables *
 ***********************/

/// Marker for external function signature.
pub const EXTERNAL_FUNCTION_MARKER: &'static str = "EXTERNAL";

/// Marker for include of another file.
pub const INCLUDE_MARKER: &'static str = "INCLUDE";

/// Names which cannot be used by variables, knots or stitches.
pub const RESERVED_KEYWORDS: &[&'static str] = &[
    "ELSE", "NOT", "TRUE", "FALSE", "AND", "OR", "FUNCTION", "RETURN",
];

#[allow(dead_code)]
/// Names of functions which are reserved in Ink.
///
/// They are not necessarily implemented yet but we reserve them for forwards compatibility.
pub const RESERVED_FUNCTIONS: &[&'static str] = &[
    "CHOICE_COUNT",
    "TURNS",
    "TURNS_SINCE",
    "SEED_RANDOM",
    "INT",
    "FLOAT",
    "FLOOR",
    "RANDOM",
    "POW",
    "LIST_ALL",
    "LIST_COUNT",
    "LIST_MIN",
    "LIST_MAX",
    "LIST_RANDOM",
    "LIST_VALUE",
    "LIST_RANGE",
    "LIST_INVERT",
];
