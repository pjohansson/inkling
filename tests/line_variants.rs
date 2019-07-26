use inkling::*;

#[test]
fn variant_sequences_can_be_nested() {
    let content = "

== start
I {once|twice|have many times} \
met with a {gentleperson|friend|{&comrade|{&bud|pal}}} \
from Nantucket. {|||||We're besties.}

+   [Continue] -> start
    
";

    let mut story = read_story_from_string(content).unwrap();
    let mut line_buffer = Vec::new();

    let choice = story
        .start(&mut line_buffer)
        .unwrap()
        .get_choices()
        .unwrap()[0]
        .clone();

    story.resume_with_choice(&choice, &mut line_buffer).unwrap();
    story.resume_with_choice(&choice, &mut line_buffer).unwrap();
    story.resume_with_choice(&choice, &mut line_buffer).unwrap();
    story.resume_with_choice(&choice, &mut line_buffer).unwrap();
    story.resume_with_choice(&choice, &mut line_buffer).unwrap();

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
