# What is Inkling?

`inkling` is a library which reads stories written in `Ink` and presents their content
to the user. It is an interface, not a game engine: it validates the script and 
returns the text, but it is up to the caller to take that text and use it however
they want in their game.


## Why use `inkling`?
*   Simple interface
*   Rust native
*   Support for non-latin alphabets in identifiers
*   Few dependencies: optional `serde` dependency for easy saving and loading, and optional 
    `rand` dependency for adding randomized features


## Why not?
*   Fewer features than Inkle's implementation of the language
*   Untested in serious work loads and large scripts: expect bugs
*   Written by a hobbyist, who cannot promise quick fixes or support