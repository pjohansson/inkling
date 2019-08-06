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

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    story.make_choice(0).unwrap();
    story.resume(&mut line_buffer).unwrap();

    story.make_choice(0).unwrap();
    story.resume(&mut line_buffer).unwrap();

    story.make_choice(0).unwrap();
    story.resume(&mut line_buffer).unwrap();

    story.make_choice(0).unwrap();
    story.resume(&mut line_buffer).unwrap();

    story.make_choice(0).unwrap();
    story.resume(&mut line_buffer).unwrap();

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

    story.start().unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 1);
    assert_eq!(&choices[0].text, "Hi!");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 2);
    assert_eq!(&choices[0].text, "Hi again!");
    assert_eq!(&choices[1].text, "Oh, you again");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
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

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I have never met a comrade from Nantucket.\n"
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I once met with a comrade from Nantucket.\n"
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I twice met with a comrade from Nantucket.\n"
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "I many times met with a comrade from Nantucket.\n"
    );
}

#[test]
fn mathematical_expressions_can_be_used_in_lines() {
    let content = "

Adding is easy: <>
{2} + {3} is {2 + 3}!

Multiplication as so: <>
2 * (3 + 5) is {2 * (3 + 5)}!

Let's nest a bit: <>
{(2 + 3 * (4 - 2 * (10 / 2 + (1 + 3 * (((4))))) - 2))} is -100!

Strings can be added, too: <>
{\"str\" + \"ing\"} is string!

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[1].text, "2 + 3 is 5!\n");
    assert_eq!(&line_buffer[3].text, "2 * (3 + 5) is 16!\n");
    assert_eq!(&line_buffer[5].text, "-100 is -100!\n");
    assert_eq!(&line_buffer[7].text, "string is string!\n");
}

#[test]
fn variables_can_be_used_in_mathematical_operations() {
    let content = "

VAR a = 3
VAR b = 5
VAR c = 13
VAR f = 3.0

Integer calculation does each step as integers, which may not be what you want.
({a} - {c}) / {a} + {b} = {(a - c) / a + b} which should be 1.66666...!

Float calculation works better:
({f} - {c}) / {f} + {b} = {(f - c) / f + b}!

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[1].text, "(3 - 13) / 3 + 5 = 2 which should be 1.66666...!\n");
    assert_eq!(&line_buffer[3].text, "(3 - 13) / 3 + 5 = 1.6666667!\n");
}

#[test]
fn variable_expressions_always_use_updated_variables() {
    let content = "

VAR a = 3
VAR b = 5

-> root

== root

{root < 2: Before | After } updating `a`: a = {a}, b = {b}, a + b = {a + b}.

+   [After setting a = 7] -> root

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[0].text, "Before updating `a`: a = 3, b = 5, a + b = 8.\n");

    story.set_variable("a", 7);
    story.make_choice(0).unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[1].text, "After updating `a`: a = 7, b = 5, a + b = 12.\n");
}
