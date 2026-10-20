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

fn graph(nodes: &[(&str, NodeKind, &str)], edges: &[(&str, &str, bool)]) -> shoryo_core::Map {
    shoryo_core::Map {
        columns: vec![],
        nodes: nodes
            .iter()
            .map(|(key, kind, question)| shoryo_core::MapNode {
                key: (*key).into(),
                kind: *kind,
                label: (*key).into(),
                question: None,
                question_id: Some(QuestionId::new(*question)),
                round: 1,
                in_review: false,
                review_mark: false,
            })
            .collect(),
        edges: edges
            .iter()
            .map(|(from, to, dashed)| shoryo_core::MapEdge {
                from: (*from).into(),
                to: (*to).into(),
                label: String::new(),
                dashed: *dashed,
            })
            .collect(),
    }
}

#[test]
fn path_preserves_prerequisite_order_and_appends_only_related_rejections() {
    use NodeKind::{Decision, Rejected};
    let map = graph(
        &[
            ("a", Decision, "qa"),
            ("b", Decision, "qb"),
            ("c", Decision, "qc"),
            ("r1", Rejected, "qb"),
            ("r2", Rejected, "other"),
        ],
        &[
            ("a", "c", false),
            ("b", "c", false),
            ("r2", "c", true),
            ("missing", "c", false),
            ("a", "b", false),
        ],
    );
    assert_eq!(map.path("c"), ["c", "b", "a", "r1"]);
    assert!(map.path("missing").is_empty());
}

#[test]
fn path_visits_cycles_and_repeated_edges_only_once() {
    let map = graph(
        &[
            ("a", NodeKind::Decision, "qa"),
            ("b", NodeKind::Decision, "qb"),
        ],
        &[
            ("a", "b", false),
            ("a", "b", false),
            ("b", "a", false),
            ("a", "a", false),
        ],
    );
    assert_eq!(map.path("b"), ["b", "a"]);
}

#[test]
fn path_uses_the_first_node_when_public_map_keys_repeat() {
    let map = graph(
        &[
            ("a", NodeKind::Decision, "first"),
            ("a", NodeKind::Decision, "second"),
            ("r1", NodeKind::Rejected, "first"),
            ("r2", NodeKind::Rejected, "second"),
        ],
        &[],
    );
    assert_eq!(map.path("a"), ["a", "r1"]);
}

#[test]
fn map_columns_align_independent_prerequisite_chains_instead_of_crossing_them() {
    let topic = topic_from(vec![
        json!({"subject":"First", "questions":[question("q1", &[]), question("q2", &[])]}),
        json!({"subject":"Second", "questions":[question("q3", &["d2"]), question("q4", &["d1"])],
            "records":{"decisions":[decision("d1", "q1", "One"), decision("d2", "q2", "Two")]}}),
        json!({"subject":"Result", "records":{"decisions":[decision("d3", "q3", "Three"), decision("d4", "q4", "Four")]}}),
    ]);
    let map = topic.map();
    let row = |column: usize, key: &str| {
        map.columns[column]
            .nodes
            .iter()
            .position(|node| node == key)
            .unwrap()
    };
    assert_eq!(row(0, "d:d1"), row(1, "d:d4"));
    assert_eq!(row(0, "d:d2"), row(1, "d:d3"));
}

#[test]
fn screen_view_covers_every_question_and_node_and_keys_sent_answers_by_round() {
    let mut rounds = chained_rounds();
    rounds[0]["questions"][1]["class"] = json!("provisional");
    let mut topic = topic_from(rounds);

    let view = topic.screen_view();

    let questions: Vec<String> = view.chains.keys().map(ToString::to_string).collect();
    assert_eq!(questions, ["q1", "q2", "q3", "q4", "q5", "q6"]);
    assert_eq!(view.chains[&QuestionId::new("q6")].len(), 2);
    assert_eq!(view.paths.len(), view.map.nodes.len());
    let mut path = view.paths["d:d2"].clone();
    path.sort();
    assert_eq!(path, ["d:d0", "d:d1", "d:d2", "d:d9"]);
    assert_eq!(view.pre_approved, [DecisionId::new("d9")]);
    assert_eq!(view.unstamped, [QuestionId::new("q6")]);
    assert_eq!(view.expected_answers, None);
    assert_eq!(
        view.sent_answers.keys().copied().collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    assert_eq!(view.sent_answers[&1].len(), 2);

    topic
        .apply(
            Operation::Stamp {
                question: QuestionId::new("q6"),
                stamped: true,
            },
            any_time(),
        )
        .unwrap();

    let sendable = topic.screen_view();
    assert!(sendable.unstamped.is_empty());
    assert_eq!(
        sendable.expected_answers.map(|answers| answers.len()),
        Some(1)
    );
}
