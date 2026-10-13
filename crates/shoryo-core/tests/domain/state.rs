use shoryo_core::{
    Answer, Choice, Class, Decision, DecisionContent, DecisionId, Origin, Question, QuestionId,
    Records, Round, RoundRecords, Timestamp, Topic,
};

fn question(id: &str) -> Question {
    Question {
        id: QuestionId::new(id),
        text: "Which store keeps the data?".into(),
        class: Class::Human,
        why_now: "Everything else reads it.".into(),
        premises: vec![DecisionId::new("d0")],
        background: vec![shoryo_core::Part::Text {
            body: "A store is where the state lives.".into(),
        }],
        options: vec![
            Choice {
                text: "One JSON file".into(),
                description: vec![shoryo_core::Part::Text {
                    body: "Plain and portable.".into(),
                }],
                recommended: true,
                consequence: "The state is one file.".into(),
            },
            Choice {
                text: "A database".into(),
                description: vec![shoryo_core::Part::Text {
                    body: "Needs a server.".into(),
                }],
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
        submitted: true,
        finished_picture: Some("a = Store".into()),
        records: RoundRecords {
            not_building: vec!["A service".into()],
            ..RoundRecords::default()
        },
        sent_at: Some(Timestamp::new("2026-10-08T04:05:00.000Z")),
        sent_as: None,
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
    topic.rounds[0].questions[0].answer.stamp = Some(shoryo_core::Stamp::Person);
    topic.rounds[0].questions[0].answer.stamped_at = topic.rounds[0].sent_at.clone();

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
    let first_ask = topic.allocate_ask_id().unwrap();
    let first_event = topic.allocate_event_id().unwrap();

    let mut reloaded = Topic::from_json(&topic.to_json()).unwrap();

    assert_ne!(reloaded.allocate_ask_id().unwrap(), first_ask);
    assert_ne!(reloaded.allocate_event_id().unwrap(), first_event);
}

#[test]
fn stored_decisions_without_content_are_refused_before_they_can_be_read() {
    let mut topic = Topic::new("", "");
    topic.records.decisions.push(Decision::new(
        DecisionId::new("d1"),
        Origin::FixRound(1),
        1,
        DecisionContent {
            name: "One file".into(),
            text: "Keep one file.".into(),
        },
    ));
    let mut stored = serde_json::to_value(&topic).unwrap();
    stored["records"]["decisions"][0]["history"] = serde_json::json!([]);

    assert!(Topic::from_json(&stored.to_string()).is_err());
    assert!(serde_json::from_value::<Decision>(stored["records"]["decisions"][0].clone()).is_err());
}

#[test]
fn stored_submissions_cannot_take_answers_from_another_round() {
    let mut topic = crate::common::topic_from(vec![
        serde_json::json!({"subject":"First", "questions":[crate::common::question("q1", &[])]}),
        serde_json::json!({"subject":"Second", "questions":[crate::common::question("q2", &[])]}),
    ]);
    crate::common::submit_current(&mut topic).unwrap();
    assert_eq!(Topic::from_json(&topic.to_json()).unwrap(), topic);

    for declared_round in [2, 99] {
        let mut stored = serde_json::to_value(&topic).unwrap();
        stored["events"][0]["round"] = serde_json::json!(declared_round);
        let error = Topic::from_json(&stored.to_string()).unwrap_err();
        assert!(matches!(error, shoryo_core::LoadError::Answer(
            shoryo_core::OperationRefusal::InvalidAnswer { question }
        ) if question == QuestionId::new("q1")));
    }
}
