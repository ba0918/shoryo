//! The shoryo command: starts a topic's server and runs the agent's commands against it.

mod client;

use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use shoryo_server::location::{TopicLocation, TopicName};
use shoryo_server::{ServeOptions, serve};

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
    /// Print the topic's whole data as JSON.
    Result { topic: String },
    /// Stop the topic's server.
    Stop { topic: String },
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
                    if let Some(warning) = &started.warning {
                        eprintln!("shoryo: warning: {warning}");
                    }
                },
            )
            .map_err(|error| error.to_string())
        }
        Commands::Round { topic, file } => {
            let body = read_input(file)?;
            let done = client::connect(&locate(&topic)?)?.post("round", body)?;
            let round = done["round"].as_u64().unwrap_or_default();
            let url = done["url"].as_str().unwrap_or_default();
            println!("Round {round} is on the screen: {url}");
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
        Commands::Result { topic } => {
            print!("{}", client::result(&locate(&topic)?)?);
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
