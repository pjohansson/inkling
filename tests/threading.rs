use inkling::*;

use std::thread;

#[test]
fn threading() {
    let content = "

Mont Blanc was a world-renowned mountain guide.
He befriended thousands of climbers and children sightseeing in Switzerland.

-> DONE

";

    let mut story = read_story_from_string(content).unwrap();

    let handle = thread::spawn(move || {
        story.start().unwrap();
        story
    });

    let mut story = handle.join().unwrap();
    let mut line_buffer = Vec::new();

    match story.resume(&mut line_buffer) {
        Ok(Prompt::Done) => {
            assert_eq!(line_buffer.len(), 2);
        }
        _ => panic!("error while reading a flat story from string"),
    }
}
