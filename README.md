# Inkling

Limited Rust implementation of the `Ink` markup/scripting language for game dialog. 

Ink is a creation of [Inkle](https://www.inklestudios.com/). For more information about the language, [see their website](https://www.inklestudios.com/ink/).

```
Using Ink you can easily write a story or create a dialog tree.

*   Branching is very simple[]: <>
    just start your line with an asterix or plus marker. 
    Want nested choices? Add more markers!
    * *     A branching choice contains all information below it[.] <>
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


## Features

Currently and likely for the foreseeable future the feature set is very limited compared to Inkle's own implementation. Available features are:

*   Knots, glue and diverts, ie. basic story structure
*   Choices, of sticky and non-sticky kinds
*   Nesting choices
*   Simple conditionals for which choices are presented, but only for checking against how many times knots have been visited
*   Tagging of lines and choices

Likely candidates for further development:

*   Fallback choices
*   Stitches
*   Line text variations: sequences, cycles, variables, conditionals
*   Includes of other files
*   De/serialization of finished stories through `serde`

Difficult features for which I doubt my skill level to implement:

*   Advanced flow control: tunnels and threads
*   Verifying that all story branches are complete
*   Mathematics and heavy logic


## Usage

See the [documentation](https://docs.rs/inkling/) or the provided example for a minimum viable story processor. 


## Contributions

Writing this has mostly been for fun and to create a simple game, hence the lack of features. Contributions are welcome!
