use shoryo_core::{
    Answer, Choice, Class, Decision, DecisionContent, DecisionId, Origin, Question, QuestionId,
    Records, Round, Topic,
};

fn question(id: &str) -> Question {
    Question {
        id: QuestionId::new(id),
        text: "Which store keeps the data?".into(),
        class: Class::Human,
        why_now: "Everything else reads it.".into(),
        premises: vec![DecisionId::new("d0")],
        background: "A store is where the state lives.".into(),
        options: vec![
            Choice {
                text: "One JSON file".into(),
                description: "Plain and portable.".into(),
                recommended: true,
                consequence: "The state is one file.".into(),
            },
            Choice {
                text: "A database".into(),
                description: "Needs a server.".into(),
                recommended: false,
                consequence: "The state needs a service.".into(),
            },
        ],
        reasks: None,
        answer: Answer::initial(0, Class::Human),
    }
}

#[test]
fn state_round_trips_through_json() {
    let mut topic = Topic::new("Data store", "Where should the data live?");
    topic.rounds.push(Round {
        number: 1,
        subject: "Storage".into(),
        review: false,
        fixes: vec![],
        questions: vec![question("q1")],
        review_conclusions: vec![],
        confirmed: vec![],
        asks: vec![],
        submitted: false,
    });
    topic.records = Records {
        decisions: vec![Decision::new(
            DecisionId::new("d0"),
            Origin::Question(QuestionId::new("q0")),
            1,
            DecisionContent {
                name: "Local only".into(),
                text: "The tool runs on the person's machine.".into(),
            },
        )],
        ..Records::default()
    };
    topic.finished_picture = Some("a = Store".into());

    let loaded = Topic::from_json(&topic.to_json()).unwrap();

    assert_eq!(loaded, topic);
}

#[test]
fn decision_content_as_of_an_earlier_round_is_its_content_then() {
    let first = DecisionContent {
        name: "One file".into(),
        text: "The state is one JSON file.".into(),
    };
    let second = DecisionContent {
        name: "Two files".into(),
        text: "The state is split in two files.".into(),
    };
    let mut decision = Decision::new(
        DecisionId::new("d1"),
        Origin::Question(QuestionId::new("q1")),
        2,
        first.clone(),
    );

    decision.revise(4, second.clone());

    assert_eq!(decision.content_as_of(1), None);
    assert_eq!(decision.content_as_of(3), Some(&first));
    assert_eq!(decision.content_as_of(4), Some(&second));
    assert_eq!(decision.current(), &second);
}

#[test]
fn ids_are_never_reused_in_a_topic() {
    let mut topic = Topic::new("Data store", "Where should the data live?");
    let first_ask = topic.allocate_ask_id();
    let first_event = topic.allocate_event_id();

    let mut reloaded = Topic::from_json(&topic.to_json()).unwrap();

    assert_ne!(reloaded.allocate_ask_id(), first_ask);
    assert_ne!(reloaded.allocate_event_id(), first_event);
}
