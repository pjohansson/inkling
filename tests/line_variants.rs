use inkling::*;

#[test]
fn variant_sequences_can_be_nested() {
    let content = "

-> start

== start
I {once|twice|have many times} \
met with a {gentleperson|friend|{&comrade|{&bud|pal}}} \
from Nantucket. {|||||We're besties.}

+   [Continue] -> start

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start(&mut line_buffer).unwrap();

    story.resume_with_choice(0, &mut line_buffer).unwrap();
    story.resume_with_choice(0, &mut line_buffer).unwrap();
    story.resume_with_choice(0, &mut line_buffer).unwrap();
    story.resume_with_choice(0, &mut line_buffer).unwrap();
    story.resume_with_choice(0, &mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I once met with a gentleperson from Nantucket.\n"
    );
    assert_eq!(
        &line_buffer[1].text,
        "I twice met with a friend from Nantucket.\n"
    );
    assert_eq!(
        &line_buffer[2].text,
        "I have many times met with a comrade from Nantucket.\n"
    );
    assert_eq!(
        &line_buffer[3].text,
        "I have many times met with a bud from Nantucket.\n"
    );
    assert_eq!(
        &line_buffer[4].text,
        "I have many times met with a comrade from Nantucket.\n"
    );
    assert_eq!(
        &line_buffer[5].text,
        "I have many times met with a pal from Nantucket. We're besties.\n"
    );
}

#[test]
fn choices_can_have_variants_in_selection_text() {
    let content = "

-> meeting

== meeting
You meet with Aaron.

+   \\{Hi|Hi again|Hello}! -> meeting
+   {meeting > 1} \\{Oh, you again|Sorry, I want some me-time right now} -> meeting
";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    let choices = story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 1);
    assert_eq!(&choices[0].text, "Hi!");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 2);
    assert_eq!(&choices[0].text, "Hi again!");
    assert_eq!(&choices[1].text, "Oh, you again");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Hello!");
    assert_eq!(&choices[1].text, "Sorry, I want some me-time right now");
}

#[test]
fn lines_can_have_conditional_content() {
    let content = "

-> root

== root

I {nantucket: {nantucket > 1: {nantucket > 2: many times | twice } | once } | have never} met {nantucket: with} a comrade from Nantucket. 

+   [Go there] -> nantucket

== nantucket
-> root

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I have never met a comrade from Nantucket.\n"
    );

    line_buffer.clear();
    story.resume_with_choice(0, &mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I once met with a comrade from Nantucket.\n"
    );

    line_buffer.clear();
    story.resume_with_choice(0, &mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I twice met with a comrade from Nantucket.\n"
    );

    line_buffer.clear();
    story.resume_with_choice(0, &mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I many times met with a comrade from Nantucket.\n"
    );
}
