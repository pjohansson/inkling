# 0.12.0

*   Breaking change: `start` does not begin the text processing. Instead, use `resume` after `start` has been called.
*   Breaking change: `resume_with_choice` has been removed and replaced with the pair of `make_choice` and `resume` methods.

## 0.12.1

*   Mathematical expressions are in. `{a + 3 * (b + c)}` in a line will evaluate to the expected value for variables `a`, `b`, and `c`.
*   Strings can be concatenated using an expression with the add operator: `"str" + "ing" == "string"`.

## 0.12.2

This release focuses on improving errors from parsing the story. Two main improvements are that parsing doesn't stop after encountering an error, but collects any it finds in a single pass before returning them all as a set. This may not catch all errors but should be an improvement over the previous version. Second is that line numbers can be accessed for all parsing and validation errors. The new helper function `print_read_error` prints these line numbers alongside the error.

*   Breaking change: error module restructured with no private structs.
*   Errors are now yielded if global variables, knots or stitches within one knot have duplicate names.
*   Lines, knots, stitches and choices have line indices associated with them to help with tracking down errors.
*   Parsing a story now returns all encountered parsing errors at once (if any), instead of just the first. This happens before validation, which is a separate step.
*   All validation errors are returned as a set instead of individually.
*   Add `print_read_error` function to describe all encountered parsing errors with line numbers.
*   Fix bug where location and variable addresses were not validated in choice lines.

## 0.12.3

*   Conditions can now use expressions on either side of a comparison: `a + 2 > b * c - 1`, and so on.

# 0.11.0

*   Resume stories with index of instead of reference to choice from the previous result.

## 0.11.1

*   Remove debugging output from used functions.

## 0.11.2

*   Validate knot and stitch addresses after parsing a story.

## 0.11.3

*   Allow fallback choices with output text (`*  [] Text content`).
*   Fix bug where glue was not respected over fallback choices.
*   Fix bug where fallback choices were not made from the knot or stitch they were found in, leading to inconsistent behavior and runtime errors.

## 0.11.4

*   Add `move_to` and `resume` methods for `Story`. These are used to respectively move the story to a different knot and resume from there.
*   Line conditionals are implemented.
*   Conditions can be nested with parenthesis and `and`/`or` connections.

## 0.11.5

*   Add reading of tags for knots. Get them with the `get_knot_tags` method for `Story`.
*   Fix bug where `serde_support` could not be activated since `FollowData` did not derive the traits.

## 0.11.6

*   Add `get_current_location` and `get_num_visited` methods for `Story`. The formers retrieves the knot and stitch name that the story is currently at. The latter the number of times a location has been visited so far in the story.
*   Add support for global variables in text. They cannot yet be used in conditions or modified in the Ink script itself, but they will be evaluated into text when encountered in the text flow.
*   Add `get_variable`/`set_variable` methods for `Story` with which to inspect or modify global variables.
*   Parse global tags for the story. Can be retrieved with the `get_story_tags` method for `Story`.

## 0.11.7

*   Variables can now be used in conditions.
*   Add `equal_to`, `greater_than` and `less_than` methods for `Variable`.

# 0.10.0

*   Add stitches to organize stories through.
*   Reorganize the `InklingError` type to separate internal from external errors. External errors are front loaded and concern user or typing errors instead of the internal machinery that is not relevant to the user (except if they occur).
*   Add optional de/serialization of stories using `serde`. Enable with feature `serde_support`.

## 0.10.1

*   Correct naming of `serde_support` feature in README.md.

## 0.10.2

*   Add support for fallback choices.

## 0.10.3

*   Improved documentation.
*   Maintenance work: node system replaced with something simpler, lines replaced with something more advanced but feature rich.

## 0.10.4

*   Add alternatives in text and choice lines. Currently regular sequences, cycles and once-only sequences are supported.
