use inkling::*;

#[test]
pub fn choices_are_filtered_after_being_picked_once_unless_sticky() {
    let content = "

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

    let result = story.start(&mut line_buffer).unwrap();

    let choices = result.get_choices().unwrap();

    assert_eq!(choices.len(), 3);
    assert_eq!(&choices[0].text, "Light your last torch.");
    assert_eq!(&choices[1].text, "Pray no grues are hiding.");
    assert_eq!(&choices[2].text, "Turn back and leave.");

    let choices = story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 2);
    assert_eq!(&choices[0].text, "Light your last torch.");
    assert_eq!(&choices[1].text, "Turn back and leave.");

    let choices = story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 2);
    assert_eq!(&choices[0].text, "Light your last torch.");
    assert_eq!(&choices[1].text, "Turn back and leave.");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 1);
    assert_eq!(&choices[0].text, "Turn back and leave.");
}

#[test]
fn choices_can_be_filtered_by_visited_knots() {
    let content = "

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

    let choices = story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Head back.");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Use your torch to light the way forward.");
    assert_eq!(&choices[1].text, "Head back.");
}

#[test]
fn choices_can_be_filtered_by_referencing_stitches_with_internal_shorthand() {
    let content = "

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

    let choices = story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Head back.");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Use your torch to light the way forward.");
    assert_eq!(&choices[1].text, "Head back.");
}

#[test]
fn choices_can_be_filtered_by_referencing_stitches_outside_the_current_knot() {
    let content = "

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

    let choices = story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Head back.");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Left");
    assert_eq!(&choices[1].text, "Right");

    let choices = story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&choices[0].text, "Use your torch to light the way forward.");
    assert_eq!(&choices[1].text, "Head back.");
}
