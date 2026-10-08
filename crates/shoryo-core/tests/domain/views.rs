use serde_json::{Value, json};
use shoryo_core::{DecisionId, NodeKind, Operation, QuestionId};

use crate::common::{any_time, decision, question, topic_from};

fn names(chain: &[shoryo_core::ChainLink]) -> Vec<&str> {
    chain.iter().map(|link| link.name.as_str()).collect()
}

/// Rounds 1–4: d0 ← q1, d1 ← q2 (on d0), d2 ← q3 (on d1, d9), d3 ← q4 (on d2);
/// d9 ← q5 in round 1. Round 5 asks q6 on d3.
fn chained_rounds() -> Vec<Value> {
    vec![
        json!({ "subject": "R1", "questions": [question("q1", &[]), question("q5", &[])] }),
        json!({
            "subject": "R2",
            "questions": [question("q2", &["d0"])],
            "records": { "decisions": [decision("d0", "q1", "Zero"), decision("d9", "q5", "Nine")] },
        }),
        json!({
            "subject": "R3",
            "questions": [question("q3", &["d1", "d9"])],
            "records": { "decisions": [decision("d1", "q2", "One")] },
        }),
        json!({
            "subject": "R4",
            "questions": [question("q4", &["d2"])],
            "records": { "decisions": [decision("d2", "q3", "Two")] },
        }),
        json!({
            "subject": "R5",
            "questions": [question("q6", &["d3", "d9"])],
            "records": { "decisions": [decision("d3", "q4", "Three")] },
        }),
    ]
}

#[test]
fn chain_follows_first_prerequisite_up_to_three_names() {
    let topic = topic_from(chained_rounds());

    let chains = topic.chains(&QuestionId::new("q6"));

    assert_eq!(names(&chains[0]), ["Three", "Two", "One"]);
}

#[test]
fn chain_lists_each_direct_prerequisite_separately() {
    let topic = topic_from(chained_rounds());

    let chains = topic.chains(&QuestionId::new("q6"));

    assert_eq!(chains.len(), 2);
    assert_eq!(names(&chains[1]), ["Nine"]);
}

#[test]
fn chain_marks_decision_in_review() {
    let mut topic = topic_from(chained_rounds());
    topic
        .apply(
            Operation::RequestReview {
                decision: DecisionId::new("d2"),
            },
            any_time(),
        )
        .unwrap();

    let chains = topic.chains(&QuestionId::new("q6"));

    let marks: Vec<bool> = chains[0].iter().map(|link| link.in_review).collect();
    assert_eq!(marks, [false, true, false]);
}

#[test]
fn decision_node_shows_the_question_it_was_decided_in() {
    let topic = topic_from(chained_rounds());

    let map = topic.map();

    let node = map.node("d:d1").unwrap();
    assert_eq!(node.label, "One");
    assert_eq!(node.question.as_deref(), Some("Question q2?"));
    assert!(map.columns[1].nodes.contains(&"d:d1".to_string()));
}

#[test]
fn current_question_sits_in_current_round_column() {
    let topic = topic_from(chained_rounds());

    let map = topic.map();

    assert_eq!(map.node("q:q6").unwrap().kind, NodeKind::Question);
    assert!(map.columns[4].nodes.contains(&"q:q6".to_string()));
    assert_eq!(map.columns[4].subject, "R5");
}

#[test]
fn map_names_edge_by_question_of_its_target() {
    let topic = topic_from(chained_rounds());

    let map = topic.map();

    let to_decision = map.edge("d:d1", "d:d2").unwrap();
    let to_question = map.edge("d:d3", "q:q6").unwrap();
    assert_eq!(to_decision.label, "Question q3?");
    assert_eq!(to_question.label, "Question q6?");
}

#[test]
fn rejected_option_hangs_dashed_from_its_question_premises() {
    let mut rounds = chained_rounds();
    rounds[4]["records"]["rejected"] =
        json!([{ "text": "Many files", "reason": "Hard to move", "question": "q3" }]);
    let topic = topic_from(rounds);

    let map = topic.map();

    let edge = map.edge("d:d1", "r:0").unwrap();
    assert!(edge.dashed);
    assert_eq!(map.node("r:0").unwrap().kind, NodeKind::Rejected);
}

#[test]
fn rejected_option_without_prerequisite_sits_alone_in_its_column() {
    let mut rounds = chained_rounds();
    rounds[4]["records"]["rejected"] =
        json!([{ "text": "A database", "reason": "Needs a server", "question": "q1" }]);
    let topic = topic_from(rounds);

    let map = topic.map();

    assert!(map.columns[0].nodes.contains(&"r:0".to_string()));
    assert!(
        map.edges
            .iter()
            .all(|edge| edge.from != "r:0" && edge.to != "r:0")
    );
}

#[test]
fn path_excludes_nodes_that_are_not_prerequisites() {
    let mut rounds = chained_rounds();
    rounds[4]["records"]["rejected"] = json!([
        { "text": "Many files", "reason": "Hard to move", "question": "q3" },
        { "text": "A database", "reason": "Needs a server", "question": "q4" },
    ]);
    let topic = topic_from(rounds);

    let mut path = topic.map().path("d:d2");
    path.sort();

    assert_eq!(path, ["d:d0", "d:d1", "d:d2", "d:d9", "r:0"]);
}

/// Round 6 revises d1; d2 rests on it directly, d3 only through d2.
fn rounds_with_revision() -> Vec<Value> {
    let mut rounds = chained_rounds();
    rounds.push(json!({
        "subject": "R6",
        "questions": [question("q7", &["d1"])],
        "records": { "decisions": [decision("d1", "q2", "One, changed")] },
        "review_conclusions": [{ "decision": "d1", "outcome": "changed" }],
    }));
    rounds
}

#[test]
fn review_mark_reaches_direct_dependents_only() {
    let topic = topic_from(rounds_with_revision());

    let map = topic.map();

    let marked: Vec<&str> = map
        .nodes
        .iter()
        .filter(|node| node.review_mark)
        .map(|node| node.key.as_str())
        .collect();
    assert_eq!(marked, ["d:d2", "q:q7"]);
}

#[test]
fn confirmed_point_clears_review_mark() {
    let mut rounds = rounds_with_revision();
    rounds.push(json!({
        "subject": "R7",
        "questions": [question("q8", &[])],
        "confirmed": [{ "decision": "d2" }],
    }));
    let topic = topic_from(rounds);

    let map = topic.map();

    assert!(!map.node("d:d2").unwrap().review_mark);
}

#[test]
fn decision_from_pre_approved_answer_is_marked_and_loses_mark_when_revised() {
    let mut provisional = question("q2", &[]);
    provisional["class"] = json!("provisional");
    let decided = json!({
        "subject": "R2",
        "questions": [question("q3", &[])],
        "records": { "decisions": [decision("d1", "q1", "One"), decision("d2", "q2", "Two")] },
    });
    let mut topic = topic_from(vec![
        json!({ "subject": "R1", "questions": [question("q1", &[]), provisional] }),
        decided,
    ]);
    assert!(!topic.decided_pre_approved(&DecisionId::new("d1")));
    assert!(topic.decided_pre_approved(&DecisionId::new("d2")));
    crate::common::submit_current(&mut topic).unwrap();

    topic
        .apply_round(crate::common::round(json!({
            "subject": "R3",
            "records": { "decisions": [decision("d2", "q2", "Two, asked again")] },
            "review_conclusions": [{ "decision": "d2", "outcome": "changed" }],
        })))
        .unwrap();

    assert!(!topic.decided_pre_approved(&DecisionId::new("d2")));
}
