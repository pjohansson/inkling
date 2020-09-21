# Inkling

[![crates.io](https://img.shields.io/crates/v/inkling.svg)](https://crates.io/crates/inkling) [![User Guide](https://img.shields.io/badge/book-guide-blue)](https://pjohansson.github.io/inkling/) [![P](https://docs.rs/inkling/badge.svg)](https://docs.rs/inkling)

Partial implementation of the *Ink* markup language for game dialogue.

Ink is a creation of [Inkle](https://www.inklestudios.com/). For more information about the language, [see their website](https://www.inklestudios.com/ink/).

```
Using Ink you can easily write a story or create a dialog tree.

*   Branching is very simple[]: <>
    just start your line with an asterix or plus marker.
    Want nested choices? Add more markers!
    * *     A branching choice contains all information below it
            of the same level or higher.
            * * *       [I see.]
    * *     Pretty cool, huh?
    - -     Use gather points like this to return all nested choices <>
            to a single path.
    * *     [Cool!] -> fin

*   You can organize the story using knots <>
    and divert (move) to them, like this:
    -> next_knot

=== next_knot ===
Simple and fun.
-> fin

=== fin ===
Ink is very powerful and has a lot more features than shown here. <>
Do note that `inkling` only implements a subset of all its features. <>
Hopefully more in the future!
-> END
```

### Why inkling?

*   Simple interface for walking through branching stories or dialog trees
*   Designed to slot into an external framework: like Inkle's implementation this is not a stand alone game engine, just a processor that will feed the story text and choices to the user
*   Rust native, no wrestling with Unity or C# integration
*   Support for non-latin alphabets in identifiers
*   Few dependencies: None required, `serde` as an optional dependency to de/serialize stories, `rand` for random sequences.

### Why not inkling?

*   Fewer features than Inkle's implementation of the language
*   Untested in serious work loads and large scripts
*   Not even alpha status, what is this???


## Features

Currently and likely for the foreseeable future the feature set is very limited compared to Inkle's own implementation. Available features are:

*   Knots, stitches, glue and diverts, ie. basic story structure
*   Choices, of sticky and non-sticky kinds, plus fallback choices
*   Nesting choices and gather points
*   Line text alternative sequences (sequences, cycle, once-only, shuffle) and conditions
*   Conditionals for displaying text and choices to user
*   Tagging of lines and choices
*   Variables in choices, conditions and text
*   Optional: De/serialization of finished stories through `serde`

Likely candidates for further development:

*   Variable modification in scripts
*   Includes of other files

Difficult features for which I doubt my skill level to implement:

*   Advanced flow control: tunnels and threads
*   Verifying that all story branches are complete


## Usage

See the [user's guide](https://pjohansson.github.io/inkling/) (under construction) and [API documentation](https://docs.rs/inkling) for more information about running the software. There is also an example minimum viable story processor which you can run with `cargo run --example player` and browse the source for. 

Enable `serde` de/serialization by activating the `serde_support` feature. This feature derives `Deserialize` and `Serialize` for all required structs.


## Contributions

Writing this has mostly been for fun and to create a simple game, hence the lack of features. Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for more details.


## License
Inkling is copyleft, licensed under [the Parity License](LICENSE-PARITY.md). See [LICENSE.md](LICENSE.md) for more details.
