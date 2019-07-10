# Inkling

Limited implementation of the `Ink` markup/scripting language for game dialog. 

Ink is a creation of [Inkle](https://www.inklestudios.com/). For more information about the language, [see their website](https://www.inklestudios.com/ink/).


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

See the documentation.


## Contributions

Writing this has mostly been for fun and to create a simple game, hence the lack of features. Contributions are welcome!
