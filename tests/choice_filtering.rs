use inkling::*;

#[test]
pub fn choices_are_filtered_after_being_picked_once_unless_sticky() {
    let content = "

-> head

== head ==
You enter a dark room.

*   Light your last torch.
    -> head
*   Pray no grues are hiding.
    -> head
+   Turn back and leave.
    -> head

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    let result = story.resume(&mut line_buffer).unwrap();

    let choices = result.get_choices().unwrap();

    assert_eq!(choices.len(), 3);
    assert_eq!(&choices[0].text, "Light your last torch.");
    assert_eq!(&choices[1].text, "Pray no grues are hiding.");
    assert_eq!(&choices[2].text, "Turn back and leave.");

    story.make_choice(1).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 2);
    assert_eq!(&choices[0].text, "Light your last torch.");
    assert_eq!(&choices[1].text, "Turn back and leave.");

    story.make_choice(1).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 2);
    assert_eq!(&choices[0].text, "Light your last torch.");
    assert_eq!(&choices[1].text, "Turn back and leave.");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 1);
    assert_eq!(&choices[0].text, "Turn back and leave.");
}

#[test]
fn choices_can_be_filtered_by_visited_knots() {
    let content = "

-> passage

== passage ==

A crossing! Which path do you take?

+   Left -> torch
+   Right -> dark_room
    
== dark_room ==
You enter a dark room.

+   {torch} Use your torch to light the way forward. 
+   Head back.
-> passage

== torch ==
In a small chamber further in you find a torch. 
You head back.
-> passage

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(1).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Head back.");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(1).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Use your torch to light the way forward.");
    assert_eq!(&choices[1].text, "Head back.");
}

#[test]
fn choices_can_be_filtered_by_referencing_stitches_with_internal_shorthand() {
    let content = "

-> exploring_the_tunnel

== exploring_the_tunnel

= passage

A crossing! Which path do you take?

+   Left -> torch
+   Right -> dark_room
    
= dark_room ==
You enter a dark room.

+   {torch} Use your torch to light the way forward. 
+   Head back.
-> passage

= torch ==
In a small chamber further in you find a torch. 
You head back.
-> passage

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(1).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Head back.");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(1).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Use your torch to light the way forward.");
    assert_eq!(&choices[1].text, "Head back.");
}

#[test]
fn choices_can_be_filtered_by_referencing_stitches_outside_the_current_knot() {
    let content = "

-> passage

== passage

A crossing! Which path do you take?

+   Left -> left_tunnel
+   Right -> dark_room
    
== dark_room ==
You enter a dark room.

+   {left_tunnel.torch} Use your torch to light the way forward. 
+   Head back.
-> passage

== left_tunnel ==
= torch
In a small chamber further in you find a torch. 
You head back.
-> passage

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(1).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Head back.");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(0).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    story.make_choice(1).unwrap();

    let choices = story
        .resume(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Use your torch to light the way forward.");
    assert_eq!(&choices[1].text, "Head back.");
}

#[test]
fn fallback_choices_are_followed_if_no_choices_remain_after_filtering() {
    let content = "

-> passage

== passage

A crossing! Which path do you take?

+   [Left] -> left_tunnel

== left_tunnel ==
{In a small chamber further in you find a torch.|This chamber used to hold a torch.}
*   [Pick up torch.] -> torch
*   ->
    <> But there is nothing left so you turn and head back.
    -> passage

== torch 
You pick the torch up and head back. 
-> passage

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "A crossing! Which path do you take?\n"
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "In a small chamber further in you find a torch.\n"
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "You pick the torch up and head back.\n"
    );
    assert_eq!(
        &line_buffer[1].text,
        "A crossing! Which path do you take?\n"
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[0].text, "This chamber used to hold a torch. ");
    assert_eq!(
        &line_buffer[1].text,
        "But there is nothing left so you turn and head back.\n"
    );
    assert_eq!(
        &line_buffer[2].text,
        "A crossing! Which path do you take?\n"
    );
}

#[test]
fn fallback_choices_may_include_text_or_direct_diverts() {
    let content = "

-> passage

== passage

A crossing! Which path do you take?

+   [Left] -> left_tunnel

== left_tunnel ==
{In a small chamber further in you find a torch.|This chamber used to hold a torch.}
*   -> torch
+   [] But there is nothing left so you turn and head back.
    -> passage

== torch 
You pick the torch up and head back. 
*   [] -> passage

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "In a small chamber further in you find a torch.\n"
    );
    assert_eq!(
        &line_buffer[1].text,
        "You pick the torch up and head back.\n"
    );
    assert_eq!(
        &line_buffer[2].text,
        "A crossing! Which path do you take?\n"
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[0].text, "This chamber used to hold a torch.\n");
    assert_eq!(
        &line_buffer[1].text,
        "But there is nothing left so you turn and head back.\n"
    );
    assert_eq!(
        &line_buffer[2].text,
        "A crossing! Which path do you take?\n"
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[0].text, "This chamber used to hold a torch.\n");
    assert_eq!(
        &line_buffer[1].text,
        "But there is nothing left so you turn and head back.\n"
    );
    assert_eq!(
        &line_buffer[2].text,
        "A crossing! Which path do you take?\n"
    );
}

#[test]
fn glue_binds_across_fallback_choices() {
    let content = "

-> passage

== passage

A crossing! Which path do you take?

+   [Left] -> left_tunnel

== left_tunnel ==
{In a small chamber further in you find a torch.|This chamber used to hold a torch.} <>
*   -> torch
*   ->
    But there is nothing left so you turn and head back.
    -> passage

== torch 
<> You pick the torch up and head back. 
-> passage

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start().unwrap();
    story.resume(&mut line_buffer).unwrap();

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer[0].text,
        "In a small chamber further in you find a torch. "
    );

    story.make_choice(0).unwrap();

    line_buffer.clear();
    story.resume(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[0].text, "This chamber used to hold a torch. ");
}
