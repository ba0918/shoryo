//! The per-user config file the screen's language and theme are kept in
//! (`docs/spec/server.md`, "設定ファイル"; `docs/spec/screen.md`, "切替は設定に残る").

use serde_json::json;

use crate::harness::{Env, files_under, first_round};

fn config_file(env: &Env) -> std::path::PathBuf {
    env.config.join("shoryo/config.toml")
}

#[test]
fn language_switch_is_written_to_the_config_file_and_survives_restart() {
    let env = Env::new();
    let server = env.start("store", &[]);

    let status = server.set_config(json!({ "language": "ja" }));
    env.run(&["stop", "store"], "");
    server.finish();
    let again = env.start("store", &[]);

    assert_eq!(status, 200);
    let written = std::fs::read_to_string(config_file(&env)).expect("the config file is written");
    assert!(written.contains(r#"language = "ja""#), "{written}");
    assert_eq!(again.config()["language"], "ja");
}

#[test]
fn switch_made_in_one_topic_applies_when_another_topic_page_opens() {
    let env = Env::new();
    let first = env.start("store", &[]);
    let second = env.start("access", &[]);

    first.set_config(json!({ "language": "ja", "theme": "dark" }));

    let opened = second.config();
    assert_eq!(opened["language"], "ja");
    assert_eq!(opened["theme"], "dark");
}

#[test]
fn broken_config_starts_with_defaults_and_is_left_unchanged() {
    let env = Env::new();
    let file = config_file(&env);
    std::fs::create_dir_all(file.parent().expect("the file has a directory"))
        .expect("the config directory can be created");
    let broken = "language = \"ja\nthis is not toml";
    std::fs::write(&file, broken).expect("the broken file can be written");

    let server = env.start("store", &[]);
    let opened = server.config();
    let refused = server.set_config(json!({ "language": "ja" }));
    env.run(&["stop", "store"], "");
    let (_, err) = server.finish();

    assert!(err.contains("config"), "no warning was printed: {err}");
    assert_eq!(opened["language"], "en");
    assert_eq!(opened["theme"], "system");
    assert_eq!(opened["unreadable"], true);
    assert_ne!(refused, 200);
    assert_eq!(
        std::fs::read_to_string(&file).expect("the file is still there"),
        broken
    );
}

#[test]
fn config_is_not_written_into_topic_data() {
    let env = Env::new();
    let server = env.start("store", &[]);
    env.run(&["round", "store"], &first_round());

    let status = server.set_config(json!({ "language": "ja", "theme": "dark" }));

    assert_eq!(status, 200);
    for path in files_under(&env.data) {
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            !content.contains("\"language\"") && !content.contains("\"theme\""),
            "{} holds the config",
            path.display()
        );
    }
}
