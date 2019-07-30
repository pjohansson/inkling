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
        Ok(Prompt::Done) => {
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
        Ok(Prompt::Done) => {
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
        Ok(Prompt::Done) => {
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
        Ok(Prompt::Done) => {
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
fn story_can_be_structured_using_stitches() {
    let content = "

== introduction

Mont Blanc was a world-renowned mountain guide.
He befriended thousands of climbers and children sightseeing in Switzerland.
-> dream.wake

== dream
= interior
GESICHT'S BEDROOM, MORNING

= wake
Gesicht is lying in his bed, eyes wide open and staring at the ceiling. 
He just woke from a nightmare.

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    match story.start(&mut line_buffer) {
        Ok(Prompt::Done) => {
            assert_eq!(line_buffer.len(), 4);
            assert_eq!(&line_buffer[3].text, "He just woke from a nightmare.\n");
        }
        _ => panic!("error while reading a flat story from string"),
    }
}

#[test]
fn stitches_can_be_diverted_to_inside_a_knot_without_the_full_address() {
    let content = "

== introduction

Mont Blanc was a world-renowned mountain guide.
He befriended thousands of climbers and children sightseeing in Switzerland.
-> dream.wake

== dream
= interior
GESICHT'S BEDROOM, MORNING

= breakfast
Before he's had the time to eat breakfast a call about a murder comes in.
As Helena probes him about leaving he suggests that they take a vacation.

= wake
Gesicht is lying in his bed, eyes wide open and staring at the ceiling. 
He just woke from a nightmare.
-> breakfast


";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    match story.start(&mut line_buffer) {
        Ok(Prompt::Done) => {
            assert_eq!(line_buffer.len(), 6);
            assert_eq!(
                &line_buffer[5].text,
                "As Helena probes him about leaving he suggests that they take a vacation.\n"
            );
        }
        Ok(_) => panic!("unexpected choice was encountered"),
        Err(err) => panic!("error while reading a flat story from string: {:?}", err),
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

    story.resume_with_choice(1, &mut line_buffer).unwrap();
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

    story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();
    story.resume_with_choice(0, &mut line_buffer).unwrap();

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

    story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();
    story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();
    story
        .resume_with_choice(1, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();
    story.resume_with_choice(0, &mut line_buffer).unwrap();

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

    story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();
    story.resume_with_choice(0, &mut line_buffer).unwrap();

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
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(
        line_buffer[0].text,
        "He entered the storage where Brau 1589 was kept.\n"
    );
    line_buffer.clear();

    assert_eq!(&choices[0].text, "“Brau 1589...”");

    story.resume_with_choice(0, &mut line_buffer).unwrap();

    assert_eq!(line_buffer[0].text, "“Brau 1589,” he said.\n");
}

#[test]
fn gathers_collect_nested_choices_in_story() {
    let content = "
    
Gesicht met with Brando. 
They travelled to his apartment by car.
*   “That was an impressive match, Brando.”
    “It's getting tougher and tougher these days,” he replied.
    * *     “Your opponents all wear the same pancreatic suits.
            “But in the ring, you're always strongest.”
            “Any match is determined by who's got the most experience.”
            He stayed silent for a moment.
            “The rest is all luck,” he continued with a wry smile.
    * *     Gesicht thought it best to wait until they were there to have a talk.
    - -     Brando turned the radio on and <>
*   Gesicht said nothing and <> 
- they stayed silent during the rest of the ride.
- -> END

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();
    line_buffer.clear();

    let choices = story
        .resume_with_choice(0, &mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(
        &choices[0].text,
        "“Your opponents all wear the same pancreatic suits."
    );

    story.resume_with_choice(0, &mut line_buffer).unwrap();

    assert_eq!(
        &line_buffer.last().unwrap().text,
        "they stayed silent during the rest of the ride.\n"
    );
}

#[test]
fn follow_with_invalid_choice_returns_error_information() {
    let content = "

North no. 2 is standing in the bed room as the old man wakes up from his dream.

*   “North no. 2? Is that you?”
*   “I thought I told you to never enter my bedroom.”
    * *     “Sir, your breakfast is ready,” North no. 2 replies.

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    story.resume_with_choice(1, &mut line_buffer).unwrap();

    // Here the same choice is used again but is not valid, because only one option
    // is available in the branch.
    let result = story.resume_with_choice(1, &mut line_buffer);

    match result {
        Err(InklingError::InvalidChoice {
            selection,
            presented_choices,
        }) => {
            assert_eq!(selection, 1);
            assert_eq!(presented_choices.len(), 1);
        }
        _ => unreachable!("the error should be present and filled with information"),
    }
}

#[test]
fn glue_binds_lines_together_without_newline_markers() {
    let content = "

“So she abandoned me ... <>
she sent me to a boarding school in England ...
<> and I never heard a thing from her again.”

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start(&mut line_buffer).unwrap();

    assert_eq!(&line_buffer[0].text, "“So she abandoned me ... ");
    assert_eq!(&line_buffer[1].text, "she sent me to a boarding school in England ... ");
    assert_eq!(&line_buffer[2].text, "and I never heard a thing from her again.”\n");
}

#[test]
fn glue_binds_across_diverts() {
    let content = "

“So she abandoned me ... <> 
-> flashback 

== flashback
she sent me to a boarding school in England ...
<> and I never heard a thing from her again.”

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    story.start(&mut line_buffer).unwrap();
    dbg!(&line_buffer);

    assert_eq!(&line_buffer[0].text, "“So she abandoned me ... ");
    assert_eq!(&line_buffer[1].text, "she sent me to a boarding school in England ... ");
    assert_eq!(&line_buffer[2].text, "and I never heard a thing from her again.”\n");
}

#[test]
fn tags_are_included_with_lines_and_choices() {
    let content = "

SCOTLAND, EURO FEDERATION # location card # europe
An old castle heaves in front of you. -> gates # description

== gates 

*   Enter it. # action

";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    let choices = story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap();

    assert_eq!(&line_buffer[0].text, "SCOTLAND, EURO FEDERATION\n");
    assert_eq!(
        line_buffer[0].tags,
        &["location card".to_string(), "europe".to_string()]
    );
    assert_eq!(&line_buffer[1].tags, &["description".to_string()]);

    assert_eq!(&choices[0].text, "Enter it.");
    assert_eq!(&choices[0].tags, &["action".to_string()]);
}
