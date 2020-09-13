//! List and examples of supported features in `ink` text documents.
//! 
//! These examples are to showcase the features as they are written in the plain `.ink` 
//! text files (although the file names do not have to end with `.ink`). Text inside of 
//! these files will appear like this:
//! 
//! ```plain
//! Example text in a file to be read.
//! ```
//! 
//! More information about these features can be found in 
//! [Inkle's guide to writing with Ink](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md).
//! However, not everything in that guide can be done with `inkling`. This is partly 
//! the reason for this document, which shows what is and is not available.
//! 
//! 
//! # Basic story elements
//! 
//! ## Text
//! 
//! Plain text is the most basic element of a story.
//! 
//! ```plain
//! I opened my notebook to a blank page, pen in hand.
//! ```
//! 
//! Text is separated into paragraphs by being on different lines.
//! 
//! ```plain
//! My hand moved towards the canvas.
//! The cold draft made a shudder run through my body.
//! A dark blot spread from where my pen was resting.
//! ```
//! 
//! Those three lines will be returned from `inkling` as separate lines, each ending with
//! a newline character.
//! 
//! ## Comments
//! 
//! The text file can contain comments, which will be ignored by `inkling` as it parses the story.
//! To write a comment, preceed the line with `//`.
//! 
//! ```plain
//! The cold could not be ignored.
//! // Unlike this line, which will be 
//! ```
//! 
//! Note that multiline comments with `/*` and `*/` are ***not*** currently supported.
//! 
//! ## Branching dialogue
//! 
//! To mark a choice in a branching dialogue, use the `*` marker.
//! 
//! ```plain
//! *   Choice 1
//! *   Choice 2 
//! ```
//! 
//! (The `+` marker can also be used, although this results in a different behavior
//! if the tree is visited again. More on this later.)
//! 
//! When `inkling` encounters one or more lines beginning with this marker, the options will 
//! be collected and returned to the user to make a choice.
//! 
//! After making a choice, the story proceeds from lines below the choice. So this story:
//! 
//! ```plain
//! A noise rang from the door.
//! *   "Hello?" I shouted.
//!     "Who's there?"
//! *   I rose from the desk and walked over.
//! ```
//! 
//! results in this "game" for the user (in this case picking the first option):
//! 
//! ```plain
//! A noise rang from the door.
//!  1: "Hello?" I shouted.
//!  2: I rose from the desk and walked over.
//! 
//! > 1
//! "Hello?" I shouted.
//! "Who's there?"
//! ```
//! 
//! ## Removing choice text from output
//! 
//! As the previous example show, by default, the choice text will be added to the 
//! text presented to the user. Text encased in square brackets `[]` will, however, 
//! be ignored. Building on the previous example: 
//! 
//! ```plain
//! *   ["Hello?" I shouted.]
//!     "Who's there?"
//! ```
//! 
//! becomes
//! 
//! ```plain
//!  1: "Hello?" I shouted.
//! 
//! > 1
//! "Who's there?"
//! ```
//! 
//! Note how the choice text is not printed below the selection.
//! 
//! ### Advanced: mixing choice and presented text
//! 
//! The square brackets also acts as a divider between choice and presented text. Any 
//! text *after* the square brackets will not appear in the choice text. Text *before*
//! the brackets will appear in both choice and output text. This makes it easy to 
//! build a simple choice text into a more presentable sentence for the story:
//! 
//! ```plain
//! *   "Hello[?"],", I shouted. "Who's there?"
//! ```
//! 
//! gives
//! 
//! ```plain
//!  1: "Hello?"
//! 
//! > 1
//! "Hello," I shouted. "Who's there?"
//! ```
//! 
//! ## Nested dialogue options
//! 
//! Dialogue branches can be nested, more or less infinitely. Just add extra `*` markers
//! to specify the depth.
//! 
//! ```plain
//! *   Choice 1
//!     * *     Choice 1.1
//!     * *     Choice 1.2
//!             * * *   Choice 1.2.1
//! *   Choice 2
//!     * *     Choice 2.1
//!     * *     Choice 2.2
//! ```
//! 
//! Any extra whitespace is just for readability. The previous example produces the exact 
//! same tree as this, much less readable, example:
//! 
//! ```plain
//! *Choice 1
//! **Choice 1.1
//! **Choice 1.2
//! ***Choice 1.2.1
//! *Choice 2
//! **Choice 2.1
//! **Choice 2.2
//! ```
//! 
//! ## Glue
//! 
//! If you want to remove the newline character from in between lines, you can use the `<>` 
//! marker which signifies *glue*. This:
//! 
//! ```plain
//! This line will <>
//! be glued to this, without creating a new paragraph.
//! ```
//! 
//! Becomes:
//! 
//! ```plain
//! This line will be glued to this, without creating a new paragraph.
//! ```
//! 
//! as will this, since glue can be put at either end:
//! 
//! ```plain
//! This line will 
//! <> be glued to this, without creating a new paragraph.
//! ```
//! 
//! For these examples glue doesn't do much, but it will be more useful once we introduce 
//! story structure features. Keep it in mind until then.
//! 
//! 
//! # Story structure
//! 
//! ## Knots
//! 
//! A story can be divided into different sections, called *knots* in `ink`. This division
//! is invisible to the user but makes it easier to write and reason about the story
//! in production. 
//! 
//! A knot is denoted by beginning the line with at least two (2) `=` signs followed by 
//! a name for the knot. On the following lines, the story text can resume.
//! 
//! ```plain
//! == stairwell
//! I made my way down the empty stairwell.
//! ```
//! 
//! The name ('stairwell' in the previous example) cannot contain spaces or non-alphanumeric 
//! symbols. Optionally, it may be followed by more `=` signs, which are not necessary but may 
//! make it easier to identify knots in the document. This is identical to the previous
//! example:
//! 
//! ```plain
//! === stairwell ===
//! I made my way down the empty stairwell.
//! ```
//! 
//! ### Non-latin characters
//! Knot names support any character as long as they are 
//! [alphanumeric](https://doc.rust-lang.org/std/primitive.char.html#method.is_alphanumeric)
//! according to the `Rust` language specification. This seems to include all languages 
//! which are recognized by UTF-8. Thus, knots (and any identifer) may contain e.g. 
//! Chinese, Japanese, Arabic, Cyrillic and other characters. Do let us know if you 
//! find any exceptions.
//! 
//! ```plain
//! === عقدة ===
//! These
//! 
//! === 매듭 ===
//! are
//! 
//! === முடிச்சு ===
//! all
//! 
//! === 結 ===
//! allowed.
//! ```
//! 
//! ## Diverts
//! *Diverts* are used to move to different parts of the story. A divert to a *knot* moves 
//! the story to continue from there. They are designated with the `->` marker followed 
//! by the destination. 
//! 
//! ```plain
//! === stairwell ===
//! The stairs creaked as I descended.
//! -> lower_floor
//! 
//! === garden ===
//! A pale moonlight illuminated the garden as I entered it.
//! -> END
//! 
//! === lower_floor ===
//! On the bottom I found an unlocked door. 
//! -> garden
//! ```
//! 
//! Diverts are automatically followed as they are encountered.
//! 
//! ### Ending the story with `-> END`
//! `END` is a destination that signifies that the story has come to, well, an end. Use 
//! `-> END` diverts for such occasions. An `ink` story is not complete unless all 
//! branches one way or another leads to an `-> END` divert: ending a story should
//! be intentional.
//! 
//! ## Diverts in choices
//! 
//! A common use of branches is to divert to other knots. 
//! 
//! ```plain
//! *   [Descend stairs] -> lower_floor
//! *   [Return to desk] 
//!     I sighed, wearily, and returned to my room.
//!     -> desk
//! 
//! === desk ===
//! As I sat by my desk, I noticed that my notebook had gone missing.
//! 
//! === lower_floor ===
//! On the bottom I found an unlocked door. 
//! ```
//! 
//! ## Stitches
//! Knots may be further subdivided into *stitches*. These are denoted by single `=` markers.
//! 
//! ```plain
//! === garden ===
//! = entering
//! A pale moonlight illuminated the garden as I entered it.
//! 
//! = well
//! The well stank of stagnant water. Is that an eel I see at the bottom?
//! ```
//! 
//! These can be diverted to using `knot.stitch` as a destination:
//! 
//! ```plain
//! -> garden.entrance
//! ```
//! 
//! Stitches within the same knot can be diverted to with only the stitch name:
//! 
//! ```plain
//! === garden ===
//! -> well
//! 
//! = entrance
//! A pale moonlight illuminated the garden as I entered it.
//! 
//! = well
//! The well stank of stagnant water. Is that an eel I see at the bottom?
//! ```
//! 
//! # Variables
//! 
//! # Metadata
//! 
//! ## Tags
//! 
//! ## Global tags
//! 
