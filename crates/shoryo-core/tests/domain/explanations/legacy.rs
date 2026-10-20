//! The legacy diagram text: its existing meanings and its limits.

use super::{accept, refused};
use serde_json::json;

#[test]
fn legacy_token_punctuation_and_repeated_grid_cells_keep_existing_meanings() {
    accept(
        json!([{"type":"diagram","title":"Tokens","role":"example","source":"a=b = Label\nc->d = Other\n| a=b | a=b |\na=b -> c->d : Done"}]),
    );
    accept(
        json!([{"type":"diagram","title":"Tokens","role":"example","source":"a=b=c\nz=Other\na=b->z"}]),
    );
}

#[test]
fn legacy_limits_have_reachable_acceptance_and_refusal_pairs() {
    let diagram = |source: String| json!([{"type":"diagram","title":"Grid","role":"example","source":source}]);
    for (n, valid) in [(1, true), (64, true), (0, false), (65, false)] {
        let source = (0..n).map(|i| format!("n{i} =\n")).collect::<String>();
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(0, true), (128, true), (129, false)] {
        let source = format!("a =\n{}", "a -> a :\n".repeat(n));
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(0, true), (64, true), (65, false)] {
        let source = format!("a =\n{}", "| a |\n".repeat(n));
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(1, true), (32, true), (33, false)] {
        let source = format!("a =\n| {}", "a |".repeat(n));
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(1, true), (64, true), (65, false)] {
        let name = "!".repeat(n);
        let source = format!("{name} =\n| {name} |\n{name} -> {name}");
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(0, true), (512, true), (513, false)] {
        let label = "x".repeat(n);
        let source = format!("a = {label}\na -> a : {label}");
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    for (n, valid) in [(32768, true), (32769, false)] {
        let source = format!("a =\n{}", " ".repeat(n - 4));
        if valid {
            accept(diagram(source));
        } else {
            refused(diagram(source));
        }
    }
    refused(diagram("a =\n| a | . |\n| a |".into()));
}
