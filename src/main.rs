//! The shoryo command: starts a topic's server and runs the agent's commands against it.

mod client;

use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use shoryo_server::location::{TopicLocation, TopicName};
use shoryo_server::{AskId, DecisionId, QuestionId, ResultSelection, ServeOptions, serve};

/// Run brainstorm rounds on a local browser screen instead of the chat.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the topic's server and keep it running; prints the page's URL.
    Start {
        topic: String,
        /// The address to listen on; anything but 127.0.0.1 serves plain HTTP to others.
        #[arg(long, default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
        bind: IpAddr,
        /// A fixed port; without it a free port is used.
        #[arg(long)]
        port: Option<u16>,
    },
    /// Put the next round on the screen; reads the round as JSON from FILE or stdin.
    Round {
        topic: String,
        file: Option<PathBuf>,
    },
    /// Wait for events on the screen and print them as JSON.
    Wait {
        topic: String,
        /// Ids of the events received last time, comma-separated.
        #[arg(long, value_delimiter = ',')]
        ack: Vec<u64>,
        /// Return with no events after this many seconds.
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Reply to an ask; reads the reply as JSON from FILE or stdin.
    Reply {
        topic: String,
        ask: u64,
        file: Option<PathBuf>,
    },
    /// End the topic.
    End { topic: String },
    /// Print current records, selected history, or the whole topic as JSON.
    Result(ResultArgs),
    /// Stop the topic's server.
    Stop { topic: String },
}

#[derive(Args)]
#[group(skip)]
#[command(group(clap::ArgGroup::new("selection").args(["current", "round", "decision", "answer", "question", "ask"]).multiple(false)))]
struct ResultArgs {
    topic: String,
    /// Print current records without questions, explanations or old decision versions.
    #[arg(long)]
    current: bool,
    /// Print one complete round, including its questions, answers and explanations.
    #[arg(long)]
    round: Option<u32>,
    /// Print one decision with its full content history.
    #[arg(long)]
    decision: Option<String>,
    /// Print only a question's sent answer and links to related asks.
    #[arg(long)]
    answer: Option<String>,
    /// Print one complete question, including its background, options and drafts.
    #[arg(long)]
    question: Option<String>,
    /// Print one ask and its reply, without expanding follow-up chains.
    #[arg(long)]
    ask: Option<u64>,
}

impl ResultArgs {
    fn into_request(self) -> (String, Option<ResultSelection>) {
        let selection = if self.current {
            Some(ResultSelection::Current {})
        } else if let Some(number) = self.round {
            Some(ResultSelection::Round { number })
        } else if let Some(id) = self.decision {
            Some(ResultSelection::Decision {
                id: DecisionId::new(id),
            })
        } else if let Some(id) = self.answer {
            Some(ResultSelection::Answer {
                id: QuestionId::new(id),
            })
        } else if let Some(id) = self.question {
            Some(ResultSelection::Question {
                id: QuestionId::new(id),
            })
        } else {
            self.ask.map(|id| ResultSelection::Ask { id: AskId(id) })
        };
        (self.topic, selection)
    }
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("shoryo: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Commands) -> Result<(), String> {
    match command {
        Commands::Start { topic, bind, port } => {
            let location = locate(&topic)?;
            serve(
                ServeOptions {
                    location,
                    bind,
                    port,
                },
                |started| {
                    println!("shoryo is serving topic {topic} at {}", started.url);
                    for warning in &started.warnings {
                        eprintln!("shoryo: warning: {warning}");
                    }
                },
            )
            .map_err(|error| error.to_string())
        }
        Commands::Round { topic, file } => {
            let body = read_input(file)?;
            let done = client::connect(&locate(&topic)?)?.post("round", body)?;
            println!("{}", round_notice(&done)?);
            Ok(())
        }
        Commands::Wait {
            topic,
            ack,
            timeout,
        } => {
            let body = serde_json::json!({ "ack": ack, "timeout": timeout }).to_string();
            let events = client::connect(&locate(&topic)?)?.post("wait", body)?;
            println!("{events}");
            Ok(())
        }
        Commands::Reply { topic, ask, file } => {
            let mut body: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&read_input(file)?)
                    .map_err(|error| format!("the reply is not a JSON object: {error}"))?;
            body.insert("ask".to_string(), ask.into());
            let body = serde_json::Value::Object(body);
            client::connect(&locate(&topic)?)?.post("reply", body.to_string())?;
            Ok(())
        }
        Commands::End { topic } => {
            client::connect(&locate(&topic)?)?.post("end", String::new())?;
            Ok(())
        }
        Commands::Result(args) => {
            let (topic, selection) = args.into_request();
            let location = locate(&topic)?;
            match selection {
                Some(selection) => println!("{}", client::selected_result(&location, &selection)?),
                None => print!("{}", client::result(&location)?),
            }
            Ok(())
        }
        Commands::Stop { topic } => {
            client::connect(&locate(&topic)?)?.post("stop", String::new())?;
            Ok(())
        }
    }
}

fn locate(topic: &str) -> Result<TopicLocation, String> {
    let name = TopicName::parse(topic).map_err(|error| error.to_string())?;
    let start = std::env::current_dir()
        .map_err(|error| format!("the current directory cannot be read: {error}"))?;
    TopicLocation::find(name, &start).map_err(|error| error.to_string())
}

fn round_notice(receipt: &serde_json::Value) -> Result<String, String> {
    let number = receipt["round"]
        .as_u64()
        .filter(|number| (1..=u64::from(u32::MAX)).contains(number))
        .ok_or_else(|| "the server's answer has no valid round number".to_string())?;
    let url = receipt["url"]
        .as_str()
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| "the server's answer has no page URL".to_string())?;
    Ok(format!("Round {number} is on the screen: {url}"))
}

fn read_input(file: Option<PathBuf>) -> Result<String, String> {
    match file {
        Some(path) => {
            std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| format!("stdin cannot be read: {error}"))?;
            Ok(input)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_notifications_require_a_valid_number_and_a_url() {
        for receipt in [
            json!({}),
            json!({"round":1}),
            json!({"url":"http://example.invalid/"}),
            json!({"round":0,"url":"http://example.invalid/"}),
            json!({"round":u64::MAX,"url":"http://example.invalid/"}),
            json!({"round":"1","url":"http://example.invalid/"}),
            json!({"round":1,"url":" "}),
        ] {
            assert!(
                round_notice(&receipt).is_err(),
                "invalid receipt accepted: {receipt}"
            );
        }
        let url = "http://example.invalid/";
        let notice = round_notice(&json!({"round":u32::MAX,"url":url})).unwrap();
        assert!(notice.contains(&u32::MAX.to_string()));
        assert!(notice.contains(url));
    }
}
