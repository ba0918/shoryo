//! The times stamps and sent rounds are kept with (`docs/spec/server.md`, "状態データ").

use serde_json::{Value, json};

use crate::harness::{Env, Server, clock_ms, first_round, stored_time};

fn first_answer(server: &Server) -> Value {
    server.view()["topic"]["rounds"][0]["questions"][0]["answer"].clone()
}

#[test]
fn stamp_time_is_kept_in_utc_and_cleared_when_the_stamp_is_lifted() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    let stamp = |stamped| {
        server.operate(json!({ "op": "stamp", "question": "q1", "stamped": stamped }));
    };

    let before = clock_ms();
    stamp(true);
    let after = clock_ms();
    let pressed = first_answer(&server)["stamped_at"].clone();
    stamp(false);
    let lifted = first_answer(&server)["stamped_at"].clone();

    let (instant, offset) = stored_time(pressed.as_str().expect("the stamp has a time"));
    assert!(
        (before..=after).contains(&instant),
        "{pressed} is not between {before} and {after}"
    );
    assert_eq!(offset, 0);
    assert_eq!(lifted, Value::Null);
}

#[test]
fn sent_round_keeps_its_sent_time_in_utc() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.stamp_all();

    let before = clock_ms();
    assert_eq!(server.send(), 200);
    let after = clock_ms();

    let sent = server.view()["topic"]["rounds"][0]["sent_at"].clone();
    let (instant, offset) = stored_time(sent.as_str().expect("the sent round has a time"));
    assert!(
        (before..=after).contains(&instant),
        "{sent} is not between {before} and {after}"
    );
    assert_eq!(offset, 0);
}
