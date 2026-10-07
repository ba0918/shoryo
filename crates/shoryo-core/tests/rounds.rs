use serde_json::{Value, json};
use shoryo_core::{
    DecisionContent, DecisionId, InReview, QuestionId, RoundInput, RoundRefusal, Topic,
};

fn option(text: &str, recommended: bool) -> Value {
    json!({
        "text": text,
        "description": format!("About {text}."),
        "recommended": recommended,
        "consequence": format!("With {text}, this follows."),
    })
}

fn question(id: &str, premises: &[&str]) -> Value {
    json!({
        "id": id,
        "text": format!("Question {id}?"),
        "class": "human",
        "why_now": "Later questions rest on it.",
        "premises": premises,
        "background": "Terms used here.",
        "options": [option("A", true), option("B", false)],
    })
}

fn decision(id: &str, question: &str, name: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "text": format!("{name}, in full."),
        "decided_by": { "question": question },
    })
}

fn round(body: Value) -> RoundInput {
    RoundInput::from_json(&body.to_string()).expect("the test writes a valid round")
}

fn first_round() -> RoundInput {
    round(json!({
        "title": "Data store",
        "original_request": "Where should the data live?",
        "subject": "Storage",
        "questions": [question("q1", &[])],
    }))
}

/// A topic whose first round is sent, with decision d1 decided by q1.
fn topic_with_submitted_round() -> Topic {
    let mut topic = Topic::new("", "");
    topic
        .apply_round(first_round())
        .expect("the first round is valid");
    topic.rounds[0].submitted = true;
    topic
        .apply_round(round(json!({
            "subject": "Format",
            "questions": [question("q2", &["d1"])],
            "records": { "decisions": [decision("d1", "q1", "One file")] },
        })))
        .expect("the second round is valid");
    topic.rounds[1].submitted = true;
    topic
}

#[test]
fn round_with_two_recommended_options_is_refused() {
    let mut topic = Topic::new("", "");
    let mut q = question("q1", &[]);
    q["options"] = json!([option("A", true), option("B", true)]);

    let refusal = topic
        .apply_round(round(json!({ "subject": "S", "questions": [q] })))
        .unwrap_err();

    assert_eq!(
        refusal,
        RoundRefusal::RecommendedCount {
            question: QuestionId::new("q1"),
            count: 2
        }
    );
    assert!(topic.rounds.is_empty());
}

#[test]
fn round_with_option_lacking_consequence_is_refused() {
    let mut topic = Topic::new("", "");
    let mut q = question("q1", &[]);
    q["options"][1]["consequence"] = json!("");

    let refusal = topic
        .apply_round(round(json!({ "subject": "S", "questions": [q] })))
        .unwrap_err();

    assert_eq!(
        refusal,
        RoundRefusal::MissingConsequence {
            question: QuestionId::new("q1"),
            option: 1
        }
    );
}

#[test]
fn round_naming_unknown_decision_is_refused() {
    let mut topic = topic_with_submitted_round();

    let refusal = topic
        .apply_round(round(json!({
            "subject": "S",
            "questions": [question("q3", &["d1", "d9"])],
        })))
        .unwrap_err();

    assert_eq!(
        refusal,
        RoundRefusal::UnknownPremise {
            question: QuestionId::new("q3"),
            decision: DecisionId::new("d9")
        }
    );
}

#[test]
fn round_may_rest_on_a_decision_it_introduces() {
    let mut topic = topic_with_submitted_round();

    let applied = topic.apply_round(round(json!({
        "subject": "S",
        "questions": [question("q3", &["d2"])],
        "records": { "decisions": [decision("d2", "q2", "Pretty JSON")] },
    })));

    assert!(applied.is_ok());
}

#[test]
fn round_reusing_a_question_id_is_refused() {
    let mut topic = topic_with_submitted_round();

    let refusal = topic
        .apply_round(round(json!({
            "subject": "S",
            "questions": [question("q1", &[])],
        })))
        .unwrap_err();

    assert_eq!(
        refusal,
        RoundRefusal::QuestionIdReused {
            question: QuestionId::new("q1")
        }
    );
}

#[test]
fn round_before_previous_is_submitted_is_refused() {
    let mut topic = Topic::new("", "");
    topic.apply_round(first_round()).unwrap();

    let refusal = topic
        .apply_round(round(json!({
            "subject": "S",
            "questions": [question("q2", &[])],
        })))
        .unwrap_err();

    assert_eq!(
        refusal,
        RoundRefusal::PreviousRoundNotSubmitted { round: 1 }
    );
    assert_eq!(topic.rounds.len(), 1);
}

#[test]
fn round_keeps_in_review_marks_set_on_the_screen() {
    let mut topic = topic_with_submitted_round();
    topic.records.in_review.push(InReview {
        decision: DecisionId::new("d1"),
        since_round: 2,
    });

    topic
        .apply_round(round(json!({
            "subject": "S",
            "questions": [question("q3", &[])],
            "records": { "in_review": [] },
        })))
        .unwrap();

    assert_eq!(
        topic.records.in_review,
        vec![InReview {
            decision: DecisionId::new("d1"),
            since_round: 2
        }]
    );
}

#[test]
fn review_conclusion_unchanged_clears_in_review() {
    let mut topic = topic_with_submitted_round();
    topic.records.in_review.push(InReview {
        decision: DecisionId::new("d1"),
        since_round: 2,
    });

    topic
        .apply_round(round(json!({
            "subject": "S",
            "questions": [question("q3", &[])],
            "review_conclusions": [{ "decision": "d1", "outcome": "unchanged" }],
        })))
        .unwrap();

    assert!(topic.records.in_review.is_empty());
}

#[test]
fn review_conclusion_changed_records_revision() {
    let mut topic = topic_with_submitted_round();
    topic.records.in_review.push(InReview {
        decision: DecisionId::new("d1"),
        since_round: 2,
    });

    topic
        .apply_round(round(json!({
            "subject": "S",
            "questions": [question("q3", &[])],
            "records": { "decisions": [decision("d1", "q1", "Two files")] },
            "review_conclusions": [{ "decision": "d1", "outcome": "changed" }],
        })))
        .unwrap();

    let revision = &topic.records.revisions[0];
    assert_eq!(revision.decision, DecisionId::new("d1"));
    assert_eq!(revision.round, 3);
    assert_eq!(
        revision.before,
        DecisionContent {
            name: "One file".into(),
            text: "One file, in full.".into()
        }
    );
    assert_eq!(revision.after.name, "Two files");
    assert!(topic.records.in_review.is_empty());
}

#[test]
fn round_after_end_reopens_topic() {
    let mut topic = topic_with_submitted_round();
    topic.ended = true;

    topic
        .apply_round(round(json!({
            "subject": "Review",
            "review": true,
            "questions": [question("q3", &[])],
        })))
        .unwrap();

    assert!(!topic.ended);
    assert_eq!(topic.rounds.len(), 3);
}

#[test]
fn round_replaces_the_records_it_carries() {
    let mut topic = topic_with_submitted_round();
    topic.records.not_building = vec!["A mobile app".into()];

    topic
        .apply_round(round(json!({
            "subject": "S",
            "questions": [question("q3", &[])],
            "records": {
                "not_building": ["A web service"],
                "decisions": [decision("d2", "q2", "Pretty JSON")],
            },
        })))
        .unwrap();

    assert_eq!(
        topic.records.not_building,
        vec!["A web service".to_string()]
    );
    let names: Vec<&str> = topic
        .records
        .decisions
        .iter()
        .map(|d| d.current().name.as_str())
        .collect();
    assert_eq!(names, ["One file", "Pretty JSON"]);
}

#[test]
fn fix_that_changed_a_decision_keeps_its_before_and_after() {
    let mut topic = topic_with_submitted_round();

    topic
        .apply_round(round(json!({
            "subject": "Review",
            "review": true,
            "fixes": [{ "text": "Named the file format", "decision": "d1" }],
            "records": { "decisions": [decision("d1", "q1", "One JSON file")] },
        })))
        .unwrap();

    let change = topic.rounds[2].fixes[0].change.as_ref().unwrap();
    assert_eq!(change.before.as_ref().unwrap().name, "One file");
    assert_eq!(change.after.name, "One JSON file");
}
