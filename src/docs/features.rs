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
//! ```
//! # use inkling::{read_story_from_string, Prompt};
//! # let content = r"
//! #
//! My hand moved towards the canvas.
//! The cold draft made a shudder run through my body.
//! A dark blot spread from where my pen was resting.
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert_eq!(buffer[0].text, "My hand moved towards the canvas.\n");
//! # assert_eq!(buffer[1].text, "The cold draft made a shudder run through my body.\n");
//! # assert_eq!(buffer[2].text, "A dark blot spread from where my pen was resting.\n");
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
//! ```
//! # use inkling::{read_story_from_string, Prompt};
//! # let content = r"
//! #
//! The cold could not be ignored.
//! // Unlike this line, which will be 
//! As will the end of this. // removed comment at end of line
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert_eq!(buffer[0].text, "The cold could not be ignored.\n");
//! # assert_eq!(buffer[1].text, "As will the end of this.\n");
//! ```
//! 
//! Note that multiline comments with `/*` and `*/` are ***not*** currently supported.
//! 
//! ## Branching dialogue
//! 
//! To mark a choice in a branching dialogue, use the `*` marker.
//! 
//! ```
//! # use inkling::{read_story_from_string, Prompt};
//! # let content = r"
//! #
//! *   Choice 1
//! *   Choice 2 
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # match story.resume(&mut buffer).unwrap() {
//! #   Prompt::Choice(choices) => {
//! #       assert_eq!(choices[0].text, "Choice 1");
//! #       assert_eq!(choices[1].text, "Choice 2");
//! #   } 
//! #   _ => unreachable!()
//! # }
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
//! ```
//! # use inkling::{read_story_from_string, Prompt};
//! # let content = r#"
//! #
//! A noise rang from the door.
//! *   "Hello?" I shouted.
//!     "Who's there?"
//! *   I rose from the desk and walked over.
//! #
//! # "#;
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # match story.resume(&mut buffer).unwrap() {
//! #   Prompt::Choice(choices) => {
//! #       assert_eq!(choices[0].text, r#""Hello?" I shouted."#);
//! #       assert_eq!(choices[1].text, r"I rose from the desk and walked over.");
//! #   } 
//! #   _ => unreachable!()
//! # }
//! # story.make_choice(0).unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert!(buffer[0].text.starts_with(r#"A noise rang from the door."#));
//! # assert!(buffer[1].text.starts_with(r#""Hello?" I shouted."#));
//! # assert!(buffer[2].text.starts_with(r#""Who's there?""#));
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
//! ```
//! # use inkling::{read_story_from_string, Prompt};
//! # let content = r#"
//! #
//! *   ["Hello?" I shouted.]
//!     "Who's there?"
//! #
//! # "#;
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # match story.resume(&mut buffer).unwrap() {
//! #   Prompt::Choice(choices) => {
//! #       assert_eq!(choices[0].text, r#""Hello?" I shouted."#);
//! #   } 
//! #   _ => unreachable!()
//! # }
//! # story.make_choice(0).unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert!(buffer[0].text.starts_with(r#""Who's there?""#));
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
//! ```
//! # use inkling::{read_story_from_string, Prompt};
//! # let content = r#"
//! #
//! *   "Hello[?"]," I shouted. "Who's there?"
//! #
//! # "#;
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # match story.resume(&mut buffer).unwrap() {
//! #   Prompt::Choice(choices) => {
//! #       assert_eq!(choices[0].text, r#""Hello?""#);
//! #   } 
//! #   _ => unreachable!()
//! # }
//! # story.make_choice(0).unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert!(buffer[0].text.starts_with(r#""Hello," I shouted. "Who's there?""#));
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
//! ```
//! # use inkling::{read_story_from_string, Prompt};
//! # let content = r"
//! #
//! *   Choice 1
//!     * *     Choice 1.1
//!     * *     Choice 1.2
//!             * * *   Choice 1.2.1
//! *   Choice 2
//!     * *     Choice 2.1
//!     * *     Choice 2.2
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # story.make_choice(0).unwrap();
//! # match story.resume(&mut buffer).unwrap() {
//! #   Prompt::Choice(choices) => {
//! #       assert_eq!(&choices[0].text, "Choice 1.1");
//! #       assert_eq!(&choices[1].text, "Choice 1.2");
//! #       story.make_choice(1).unwrap();
//! #       match story.resume(&mut buffer).unwrap() {
//! #           Prompt::Choice(choices) => {
//! #               assert_eq!(&choices[0].text, "Choice 1.2.1");
//! #           }
//! #           _ => unreachable!()
//! #       }
//! #   }
//! #   _ => unreachable!()
//! # }
//! ```
//! 
//! Any extra whitespace is just for readability. The previous example produces the exact 
//! same tree as this, much less readable, example:
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content_nowhitespace = r"
//! #
//! *Choice 1
//! **Choice 1.1
//! **Choice 1.2
//! ***Choice 1.2.1
//! *Choice 2
//! **Choice 2.1
//! **Choice 2.2
//! #
//! # ";
//! # let content_whitespace = r"
//! #
//! # *   Choice 1
//! #     * *     Choice 1.1
//! #     * *     Choice 1.2
//! #             * * *   Choice 1.2.1
//! # *   Choice 2
//! #     * *     Choice 2.1
//! #     * *     Choice 2.2
//! #
//! # ";
//! #
//! # let story_nowhitespace = read_story_from_string(content_nowhitespace).unwrap();
//! # let story_whitespace = read_story_from_string(content_whitespace).unwrap();
//! # assert_eq!(format!("{:?}", story_nowhitespace), format!("{:?}", story_whitespace));
//! ```
//! 
//! ## Glue
//! 
//! If you want to remove the newline character from in between lines, you can use the `<>` 
//! marker which signifies *glue*. This:
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! #
//! This line will <>
//! be glued to this, without creating a new paragraph.
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert!(!buffer[0].text.ends_with("\n"));
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
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! #
//! This line will 
//! <> be glued to this, without creating a new paragraph.
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert!(!buffer[0].text.ends_with("\n"));
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
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! #
//! == stairwell
//! I made my way down the empty stairwell.
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # assert!(story.move_to("stairwell", None).is_ok());
//! ```
//! 
//! The name ('stairwell' in the previous example) cannot contain spaces or non-alphanumeric 
//! symbols. Optionally, it may be followed by more `=` signs, which are not necessary but may 
//! make it easier to identify knots in the document. This is identical to the previous
//! example:
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! #
//! === stairwell ===
//! I made my way down the empty stairwell.
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # assert!(story.move_to("stairwell", None).is_ok());
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
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! #
//! === عقدة ===
//! These
//! 
//! === 매듭 ===
//! are
//! 
//! === गांठ ===
//! all
//! 
//! === 結 ===
//! allowed.
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # assert!(story.move_to("عقدة", None).is_ok());
//! # assert!(story.move_to("매듭", None).is_ok());
//! # assert!(story.move_to("गांठ", None).is_ok());
//! # assert!(story.move_to("結", None).is_ok());
//! ```
//! 
//! ## Stitches
//! Knots may be further subdivided into *stitches*. These are denoted by single `=` markers.
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! #
//! === garden ===
//! = entrance
//! A pale moonlight illuminated the garden.
//! 
//! = well
//! The well stank of stagnant water. Is that an eel I see at the bottom?
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # assert!(story.move_to("garden", Some("entrance")).is_ok());
//! # assert!(story.move_to("garden", Some("well")).is_ok());
//! ```
//! 
//! ## Diverts
//! *Diverts* are used to move to different parts of the story. A divert to a *knot* moves 
//! the story to continue from there. They are designated with the `->` marker followed 
//! by the destination. 
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! # -> stairwell
//! #
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
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert_eq!(story.get_current_location().unwrap(), ("garden".to_string(), None));
//! ```
//! 
//! Diverts are automatically followed as they are encountered.
//! 
//! ### Diverts to stitches
//! 
//! Stitches inside knots can be diverted to using `knot.stitch` as a destination:
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! #
//! -> garden.entrance
//! #
//! # === garden ===
//! # = well
//! # Unreachable.
//! # = entrance 
//! # A pale moonlight illuminated the garden.
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert!(buffer[0].text.starts_with("A pale moonlight illuminated the garden."));
//! ```
//! 
//! Stitches within the same knot can be diverted to with only the stitch name:
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! # -> garden
//! #
//! === garden ===
//! -> well
//! 
//! = entrance
//! A pale moonlight illuminated the garden.
//! 
//! = well
//! The well stank of stagnant water. 
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert!(buffer[0].text.starts_with("The well stank of stagnant water."));
//! ```
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
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r"
//! #
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
//! #
//! # ";
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # story.make_choice(1).unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert_eq!(story.get_current_location().unwrap(), ("desk".to_string(), None));
//! ```
//! 
//! # Variables
//! 
//! Global variables can be declared in the script using the `VAR` keyword. They 
//! must be declared before the first knot.
//! 
//! ```
//! # use inkling::{read_story_from_string, Variable};
//! # let content = r#"
//! #
//! VAR a_float = 1.0
//! VAR a_int = 2
//! VAR a_bool = false
//! VAR a_string = "A String"
//! VAR a_destination = "-> stairwell"
//! #
//! # "#;
//! # let story = read_story_from_string(content).unwrap();
//! # assert_eq!(story.get_variable("a_float").unwrap(), Variable::Float(1.0));
//! # assert_eq!(story.get_variable("a_int").unwrap(), Variable::Int(2));
//! # assert_eq!(story.get_variable("a_bool").unwrap(), Variable::Bool(false));
//! # assert_eq!(story.get_variable("a_string").unwrap(), Variable::String("A String".to_string()));
//! # assert!(story.get_variable("a_destination").is_ok());
//! ```
//! 
//! As shown in this example, the variable type is automatically assigned from 
//! the given value. Once assigned, a variable's type cannot be changed.
//! 
//! ## Using variables in text
//! 
//! Variables can be inserted into text by enclosing them in curly braces.
//! 
//! ```
//! # use inkling::{read_story_from_string, Variable};
//! # let content = r#"
//! #
//! VAR time = 11
//! VAR moon = "gloomy"
//! 
//! The time was {time}. A {moon} moon illuminated the room.
//! #
//! # "#;
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # assert_eq!(buffer[0].text, "The time was 11. A gloomy moon illuminated the room.\n");
//! ```
//! 
//! ## Variable assignment
//! 
//! **It is not currently possible to assign variables in the script. Use `Story::set_variable`.**
//! 
//! ## Constant variables
//! 
//! Constant variables, whose values cannot be changed, are declared using the `CONST` keyword.
//! 
//! ```
//! # use inkling::{read_story_from_string, Variable};
//! # let content = r#"
//! #
//! CONST name = "d'Artagnan" // Constant variable, cannot be modified
//! VAR rank = "Capitaine"    // Non-constant variable, can be changed
//! #
//! # "#;
//! # let mut story = read_story_from_string(content).unwrap();
//! # assert_eq!(story.get_variable("name").unwrap(), Variable::from("d'Artagnan"));
//! # assert!(story.set_variable("name", "Aramis").is_err());
//! ```
//! 
//! # Metadata
//! 
//! Information about the story, knots or even individual lines can be marked with *tags*. All tags
//! begin with the `#` marker.
//! 
//! Tags are stored as pure strings and can thus be of any form you want. `inkling` assigns no 
//! meaning to them on its own, it's for you as the user to decide how to treat them.
//! 
//! ## Global tags
//! 
//! Tags in the preamble are global story tags. Here you can typically mark up metadata for the script.
//! 
//! ```
//! # use inkling::{read_story_from_string, Variable};
//! # let content = r#"
//! #
//! ## title: Inkling
//! ## author: Petter Johansson
//! #
//! # "#;
//! # let story = read_story_from_string(content).unwrap();
//! # let tags = story.get_story_tags();
//! # assert_eq!(&tags[0], "title: Inkling");
//! # assert_eq!(&tags[1], "author: Petter Johansson");
//! ```
//! 
//! ## Knot tags
//! 
//! Tags encountered in a knot before any content is parsed as tags belonging to that knot.
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r#"
//! #
//! === stairwell ===
//! ## sound: blowing_wind.ogg
//! ## dark, quiet, dangerous
//! I made my way down the empty stairwell.
//! #
//! # "#;
//! # let story = read_story_from_string(content).unwrap();
//! # let tags = story.get_knot_tags("stairwell").unwrap();
//! # assert_eq!(&tags[0], "sound: blowing_wind.ogg");
//! # assert_eq!(&tags[1], "dark, quiet, dangerous");
//! ```
//! 
//! ## Line tags
//! 
//! Lines can be tagged by adding the tag after the line content. Multiple tags can 
//! be set, separated by additional '#' markers.
//! 
//! ```
//! # use inkling::read_story_from_string;
//! # let content = r#"
//! #
//! A pale moonlight illuminated the garden. # sound: crickets.ogg
//! The well stank of stagnant water. # smell, fall # sound: water_drip.ogg
//! #
//! # "#;
//! # let mut story = read_story_from_string(content).unwrap();
//! # let mut buffer = Vec::new();
//! # story.start().unwrap();
//! # story.resume(&mut buffer).unwrap();
//! # let tags1 = &buffer[0].tags;
//! # let tags2 = &buffer[1].tags;
//! # assert_eq!(&tags1[0], "sound: crickets.ogg");
//! # assert_eq!(&tags2[0], "smell, fall");
//! # assert_eq!(&tags2[1], "sound: water_drip.ogg");
//! ```
//! 
