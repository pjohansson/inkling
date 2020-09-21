# Short example

This section provides an example of a script and how to read it into your program
with `inkling`.

## Script

```ink
// This is an `ink` script, saved as 'story.ink'

A single candle flickered by my side.
Pen in hand I procured a blank letter.

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

## Rust code

```rust,ignore
# extern crate inkling;
use std::fs::read_to_string;
use inkling::read_story_from_string;

// Read the script into memory
let story_content = read_to_string("story.ink").unwrap();

// Read the story from the script
let mut story = read_story_from_string(&story_content).unwrap();
```

The next chapter will explain how to proceed from here.