# 0.10.0

*   Add stitches to organize stories through.
*   Reorganize the `InklingError` type to separate internal from external errors. External errors are front loaded and concern user or typing errors instead of the internal machinery that is not relevant to the user (except if they occur).
*   Add optional de/serialization of stories using `serde`. Enable with feature `serde_support`.
