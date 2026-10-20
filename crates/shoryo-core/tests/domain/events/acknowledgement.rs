//! Events stay pending until the agent says it has received them.

use super::{d, q, topic_on_a_round};
use crate::common::{any_time, submit_current};
use shoryo_core::Operation;

#[test]
fn acknowledged_events_are_not_returned() {
    let mut topic = topic_on_a_round();
    topic
        .apply(Operation::RequestReview { decision: d("d1") }, any_time())
        .unwrap();
    submit_current(&mut topic).unwrap();
    let first = topic.pending_events()[0].id;

    topic.acknowledge(&[first]);

    let left: Vec<_> = topic.pending_events().iter().map(|e| e.id).collect();
    assert_eq!(left.len(), 1);
    assert!(!left.contains(&first));
}

#[test]
fn acknowledging_duplicates_and_unknown_events_preserves_pending_order() {
    let mut topic = topic_on_a_round();
    for _ in 0..5 {
        topic
            .apply(Operation::SwapClass { question: q("q2") }, any_time())
            .unwrap();
    }
    let before = topic.pending_events().to_vec();
    topic.acknowledge(&[]);
    assert_eq!(topic.pending_events(), before);
    topic.acknowledge(&[
        before[3].id,
        shoryo_core::EventId(99),
        before[1].id,
        before[3].id,
    ]);
    assert_eq!(
        topic.pending_events(),
        [before[0].clone(), before[2].clone(), before[4].clone()]
    );
    topic.acknowledge(&[before[2].id]);
    assert_eq!(
        topic.pending_events(),
        [before[0].clone(), before[4].clone()]
    );
    topic
        .apply(Operation::SwapClass { question: q("q2") }, any_time())
        .unwrap();
    assert!(topic.pending_events().last().unwrap().id > before[4].id);
}
