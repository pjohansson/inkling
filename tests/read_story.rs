use inkling::*;

#[test]
fn story_can_be_read_with_unnamed_knots() {
    let content = "

Mont Blanc was a world-renowned mountain guide.
He befriended thousands of climbers and children sightseeing in Switzerland.

-> DONE

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    match story.start(&mut line_buffer) {
        Ok(StoryAction::Done) => {
            assert_eq!(line_buffer.len(), 2);
        }
        _ => panic!("error while reading a flat story from string"),
    }
}

#[test]
fn story_starts_from_top_even_if_knot_is_unnamed() {
    let content = "

Mont Blanc was a world-renowned mountain guide.
He befriended thousands of climbers and children sightseeing in Switzerland.
-> dream

== dream ==
GESICHT'S BEDROOM, MORNING

Gesicht is lying in his bed, eyes wide open and staring at the ceiling. 
He just woke from a nightmare.

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    match story.start(&mut line_buffer) {
        Ok(StoryAction::Done) => {
            assert_eq!(line_buffer.len(), 5);
            assert_eq!(&line_buffer[4].text, "He just woke from a nightmare.\n");
        }
        _ => panic!("error while reading a flat story from string"),
    }
}

#[test]
fn story_can_start_with_named_knot() {
    let content = "

== dream ==
GESICHT'S BEDROOM (MORNING)

Gesicht is lying in his bed, eyes wide open and staring at the ceiling. 
He just woke from a nightmare.

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    match story.start(&mut line_buffer) {
        Ok(StoryAction::Done) => {
            assert_eq!(line_buffer.len(), 3);
            assert_eq!(&line_buffer[2].text, "He just woke from a nightmare.\n");
        }
        _ => panic!("error while reading a flat story from string"),
    }
}

#[test]
fn story_can_divert_at_will_between_unordered_knots() {
    let content = "

== murder ==
SCENE OF MURDER (TRASHED APARTMENT, DAYTIME)

Gesicht arrives at a grotesque murder scene.

-> cops_hold_him

== investigate_body ==
The body is lying face down in a pool of blood. 
A desk lamp and piece of broken wood have been stuck to his head.
They mimic the appearance of antlers.

== cops_hold_him
The lead detective stop him as he enters the room.
He identifies himself as being from Europol and passes the barrier.

-> investigate_body

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    match story.start(&mut line_buffer) {
        Ok(StoryAction::Done) => {
            assert_eq!(line_buffer.len(), 7);
            assert_eq!(
                &line_buffer[6].text,
                "They mimic the appearance of antlers.\n"
            );
        }
        _ => panic!("error while reading a flat story from string"),
    }
}

#[test]
fn story_follows_choices_by_the_user() {
    let content = "

Gesicht corners the fugitive and slams him to the ground.
-> cornered 

== cornered ==
He points his gun hand in the fugitive's face point blank.
“Now, I can read you your rights and you'll let me arrest you.
“Or, I can shoot you with hypno-gas and you'll lose consciousness.
“Take your pick! Which is it?!”

*   Sirens approach and the police take him in.
*   The fugitive desperately fights back.
    -> fight

== fight ==
The fight barely lasts a moment before Gesicht sedates him with a large dose of gas.
“What was the point in all that?”
";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    let choices = story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(choices.len(), 2);
    assert_eq!(
        &choices[0].text,
        "Sirens approach and the police take him in."
    );
    assert_eq!(&choices[1].text, "The fugitive desperately fights back.");

    story.resume_with_choice(&choices[1], &mut line_buffer).unwrap();
    assert_eq!(
        line_buffer.last().unwrap().text,
        "“What was the point in all that?”\n"
    );
}

#[test]
fn following_a_choice_adds_a_copy_of_the_choice_line_to_the_buffer() {
    let content = "
*   Gesicht took the fugitive in.
";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    let choices = story.start(&mut line_buffer).unwrap().get_choices().unwrap();
    story.resume_with_choice(&choices[0], &mut line_buffer).unwrap();

    assert_eq!(line_buffer.len(), 1);
    assert_eq!(&line_buffer[0].text, "Gesicht took the fugitive in.\n");
}

#[test]
fn choices_can_nest_into_multiple_levels() {
    let content = "

Gesicht knocks on the door. 
A robot in a frilly apron welcomes him in.

*   He steps in and informs the widow about her husband's death<>
    * *     He then offers his condolences.
    * *     He gives her husband's memory chip to her.
            * * *   He helps the widow insert it.
*   He informs about the death and leave the apartment.
";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    let choices = story.start(&mut line_buffer).unwrap().get_choices().unwrap();
    let choices = story.resume_with_choice(&choices[0], &mut line_buffer).unwrap().get_choices().unwrap();
    let choices = story.resume_with_choice(&choices[1], &mut line_buffer).unwrap().get_choices().unwrap();
    story.resume_with_choice(&choices[0], &mut line_buffer).unwrap();

    assert_eq!(
        line_buffer.last().unwrap().text,
        "He helps the widow insert it.\n"
    );
}

#[test]
fn choices_can_divert_in_their_lines() {
    let content = "

Gesicht notices that a destroyed patrol bot is being thrown away.

*   Question the garbage worker -> question
*   Ignore it.

== question ==
“Excuse me sir, isn't that a patrol bot?”

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    let choices = story.start(&mut line_buffer).unwrap().get_choices().unwrap();
    story.resume_with_choice(&choices[0], &mut line_buffer).unwrap();

    assert_eq!(
        line_buffer.last().unwrap().text,
        "“Excuse me sir, isn't that a patrol bot?”\n"
    );
}

#[test]
fn diverts_are_glue_and_add_single_whitespace_when_following_a_story() {
    let content = "

“We all loved Mont Blanc.
“The memorial service will take place in three days ...
“This place will be filled with tens of thousands of people,-> view_over_stadium

== view_over_stadium ==
probably several hundred thousands.
“all mourning the death of mont blanc.”

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start(&mut line_buffer).unwrap();

    assert!(line_buffer[2].text.ends_with(' '));
    assert_eq!(line_buffer[3].text, "probably several hundred thousands.\n");
}

#[test]
fn choices_with_bracket_text_gives_different_selection_and_displayed_lines() {
    let content = "

Gesicht descended into the prison.
The elevator doors swung open.
*   [Enter]He entered the storage where Brau 1589 was kept.
    Further in he encountered the robot.
    * *    “Brau 1589[...”],” he said.
    
";
    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    let choices = story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();
    line_buffer.clear();

    assert_eq!(&choices[0].text, "Enter");

    let choices = story
        .resume_with_choice(&choices[0], &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();
    assert_eq!(
        line_buffer[0].text,
        "He entered the storage where Brau 1589 was kept.\n"
    );
    line_buffer.clear();

    assert_eq!(&choices[0].text, "“Brau 1589...”");

    story.resume_with_choice(&choices[0], &mut line_buffer).unwrap();
    assert_eq!(line_buffer[0].text, "“Brau 1589,” he said.\n");
}
