//! Changes pushed to open pages without a reload.

use std::io::{BufRead, BufReader};

use serde_json::json;

use crate::harness::{Env, Server, events, first_round, text};

fn read_page_stream_until(
    server: &Server,
    needle: &str,
    ready: std::sync::mpsc::Sender<()>,
) -> bool {
    let mut response = Server::agent()
        .get(format!("{}api/events", server.url))
        .call()
        .expect("the event stream opens");
    let reader = BufReader::new(response.body_mut().as_reader());
    let mut ready = Some(ready);
    for line in reader.lines() {
        let Ok(line) = line else { return false };
        if line.is_empty()
            && let Some(ready) = ready.take()
        {
            ready
                .send(())
                .expect("the test is waiting for the initial snapshot");
        }
        if line.contains(needle) {
            return true;
        }
    }
    false
}

#[test]
fn reply_reaches_open_page_without_reload() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());
    server.operate(json!({ "op": "ask", "question": "q1", "text": "Explain more" }));
    let ask = events(&env.run(&["wait", "store", "--timeout", "1"], ""))[0]["ask"].to_string();

    std::thread::scope(|scope| {
        let (ready, received) = std::sync::mpsc::channel();
        let page = scope.spawn(|| read_page_stream_until(&server, "It is one file", ready));
        received
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the page receives its initial snapshot");
        let reply = env.run(
            &["reply", "store", &ask],
            &json!({ "parts": [{"type":"text","body":"It is one file on disk."}] }).to_string(),
        );
        assert!(reply.status.success(), "{}", text(&reply));
        assert!(page.join().expect("the page reader finishes"));
    });
}

#[test]
fn two_open_pages_receive_the_same_change() {
    let env = Env::new();
    let server = env.start("store", &[]);

    std::thread::scope(|scope| {
        let (ready, received) = std::sync::mpsc::channel();
        let second_ready = ready.clone();
        let first = scope.spawn(|| read_page_stream_until(&server, "Which store keeps", ready));
        let second =
            scope.spawn(|| read_page_stream_until(&server, "Which store keeps", second_ready));
        for _ in 0..2 {
            received
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("both pages receive their initial snapshots");
        }
        env.run(&["round", "store"], &first_round());
        assert!(first.join().expect("the first page finishes"));
        assert!(second.join().expect("the second page finishes"));
    });
}
