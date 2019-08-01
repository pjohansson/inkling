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

*   Conditions can be nested with parenthesis and `and`/`or` connections.

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
