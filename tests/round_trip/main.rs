//! The round trip between the agent's commands, the server and the page API, run against
//! the built binary (`docs/spec/server.md`, "起動", "往復", "状態データ", "記録の置き場所と寿命").

mod answer_results;
mod commands;
mod config;
mod explanations;
mod free_text;
mod harness;
mod large_responses;
mod lifecycle;
mod page_stream;
mod selective_results;
mod skill_examples;
mod times;
mod wait;
