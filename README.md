# Inkling

[![User Guide](https://img.shields.io/badge/book-guide-blue)](https://pjohansson.github.io/inkling/) [![Documentation](https://docs.rs/inkling/badge.svg)](https://docs.rs/inkling) [![crates.io](https://img.shields.io/crates/v/inkling.svg)](https://crates.io/crates/inkling)

Partial implementation of the *Ink* markup language for game dialogue.

Ink is a creation of [Inkle](https://www.inklestudios.com/). For more information about the language, [see their website](https://www.inklestudios.com/ink/).

```
A single candle flickered by my side.
Pen in hand I made my decision and procured a blank letter.

*   "Dear Guillaume"
    Sparing the more unfavorable details from him, I requested his aid.
    -> guillaume_arrives

*   "To the Fiendish Impostor"
    -> write_to_laurent

=== guillaume_arrives ===
A few days later my servant informed me of Guillaume's arrival. 
I met with him in the lounge.

=== write_to_laurent ===
The letter was spiked with insults and veiled threats.
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


## Usage

See the [User Guide](https://pjohansson.github.io/inkling/) and [documentation](https://docs.rs/inkling) for more information about running the software. There is also an example minimum viable story processor which you can run with `cargo run --example player` and browse the source for. 


## Contributions

Writing this has mostly been for fun and to create a simple game, hence the lack of features. Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for more details.


## License
Inkling is copyleft, licensed under [the Parity License](LICENSE-PARITY.md). See [LICENSE.md](LICENSE.md) for more details.
