# 0.10.0

*   Add stitches to organize stories through.
*   Reorganize the `InklingError` type to separate internal from external errors. External errors are front loaded and concern user or typing errors instead of the internal machinery that is not relevant to the user (except if they occur).
*   Add optional de/serialization of stories using `serde`. Enable with feature `serde_support`.

## 0.10.1

*   Correct naming of `serde_support` feature in README.md

## 0.10.2

*   Add support for fallback choices

## 0.10.3

*   Improved documentation
*   Maintenance work: node system replaced with something simpler, lines replaced with something more advanced but feature rich

## 0.10.4

*   Add alternatives in text and choice lines. Currently regular sequences, cycles and once-only sequences are supported.
