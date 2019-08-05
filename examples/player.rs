use std::{
    env::current_dir,
    fs::read_to_string,
    io,
    path::{Path, PathBuf},
    process::exit,
};

use inkling::*;

fn main() -> Result<(), io::Error> {
    let base_dir = current_dir().unwrap();

    let mut assets_dir = base_dir.clone();
    assets_dir.push("examples");
    assets_dir.push("assets");

    let path: PathBuf = [assets_dir.as_path(), Path::new("story.ink")]
        .iter()
        .collect();
    let story = read_story(&path)?;

    match play_story(story) {
        Ok(_) => println!("FIN\n"),
        Err(err) => {
            eprintln!("error: {}", err);
            exit(1);
        }
    }

    Ok(())
}

fn play_story(mut story: Story) -> Result<(), InklingError> {
    let mut line_buffer = Vec::new();
    story.start()?;

    while let Prompt::Choice(choices) = story.resume(&mut line_buffer)? {
        print_lines(&line_buffer);
        line_buffer.clear();

        let choice = ask_user_for_choice(&choices).unwrap_or_else(|| {
            println!("Exiting program.");
            exit(0);
        });

        println!("");
        story.make_choice(choice)?;
    }

    Ok(())
}

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
        io::stdin().read_line(&mut input).unwrap();

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

fn print_lines(lines: &LineBuffer) {
    for line in lines {
        print!("{}", line.text);

        if line.text.ends_with('\n') {
            print!("\n");
        }
    }
}

fn read_story(path: &Path) -> Result<Story, io::Error> {
    let contents = read_to_string(path)?;
    Ok(read_story_from_string(&contents).unwrap())
}
