# Example: Text adventure

This page contains a sample implementation of a simple story reader using `inkling`.
It only uses plain text and a terminal.

The full code can be found as the `player.rs` example on [Github](https://github.com/pjohansson/inkling).


## Design

The player requires this functionality:

*   Reading a story from a file
*   A game loop
*   Presenting the text to the player
*   Asking the player for a choice at branches

## Reading

A simple function that attempts to read the story from a file at a given path.
If errors are encountered they should be handled.

```rust
# extern crate inkling;
# use std::io::Write;
use inkling::{error::parse::print_read_error, read_story_from_string, Story};

fn read_story(path: &std::path::Path) -> Result<Story, std::io::Error> {
    let content = std::fs::read_to_string(path)?;

    match read_story_from_string(&content) {
        Ok(story) => Ok(story),
        Err(error) => {
            // If the story could not be parsed, write the list of errors to stderr
            write!(
                std::io::stderr(),
                "{}",
                print_read_error(&error).unwrap()
            )
            .unwrap();

            std::process::exit(1);
        }
    }
}
```

## Game loop

The main loop implements the [standard pattern](./design-intent.md).

```rust
# extern crate inkling;
use inkling::{InklingError, Prompt, Story};

fn play_story(mut story: Story) -> Result<(), InklingError> {
    let mut line_buffer = Vec::new();

    while let Prompt::Choice(choices) = story.resume(&mut line_buffer)? {
        print_lines(&line_buffer);
        line_buffer.clear();

        let choice = ask_user_for_choice(&choices).unwrap_or_else(|| {
            println!("Exiting program.");
            std::process::exit(0);
        });

        println!("");
        story.make_choice(choice)?;
    }

    Ok(())
}
#
# // Mock the following functions
# fn print_lines(buffer: &inkling::LineBuffer) { unimplemented!(); }
# fn ask_user_for_choice(choices: &[inkling::Choice]) -> Option<usize> { unimplemented!(); }
```

## Printing story text

Simply iterate through the list of lines and print the text. Add an extra
newline if there is a paragraph break.

```rust
# extern crate inkling;
use inkling::LineBuffer;

fn print_lines(lines: &LineBuffer) {
    for line in lines {
        print!("{}", line.text);

        if line.text.ends_with('\n') {
            print!("\n");
        }
    }
}
```

## Asking the player for a choice

Print the available choices one by one, then ask for a selection.

```rust
# extern crate inkling;
# use std::io;
use inkling::Choice;

fn ask_user_for_choice(choices: &[Choice]) -> Option<usize> {
    println!("Choose:");

    for (i, choice) in choices.iter().enumerate() {
        println!("  {}. {}", i + 1, choice.text);
    }

    println!("     ---");
    println!("  0. Exit story");
    println!("");

    let index = get_choice(choices.len())?;
    Some(index)
}

fn get_choice(num_choices: usize) -> Option<usize> {
    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        match input.trim().parse::<usize>() {
            Ok(0) => {
                return None;
            }
            Ok(i) if i > 0 && i <= num_choices => {
                return Some(i - 1);
            }
            _ => {
                println!("Not a valid option, try again:");
            }
        }
    }
}
```