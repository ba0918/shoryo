use serde_json::{Value, json};
use shoryo_core::{RoundInput, Topic};

fn input(parts: Value) -> Value {
    json!({"subject":"Reading", "questions":[{
        "id":"q1", "text":"Which approach?", "class":"human", "why_now":"Choose a direction.",
        "background":parts, "options":[{"text":"A", "description":[], "recommended":true,"consequence":"Continue."}]
    }]})
}

#[test]
fn ordered_parts_preserve_all_metadata() {
    let parts = json!([
        {"type":"text","body":"Before\n<script>"},
        {"type":"code","body":"  run(); \n","language":"unknown","role":"example","title":"A sample"},
        {"type":"text","body":"After"}
    ]);
    let mut topic = Topic::new("", "");
    let round = RoundInput::from_json(&input(parts.clone()).to_string()).unwrap();
    topic.apply_round(round).unwrap();
    let stored: Value = serde_json::from_str(&topic.to_json()).unwrap();
    assert_eq!(stored["rounds"][0]["questions"][0]["background"], parts);
    assert_eq!(Topic::from_json(&topic.to_json()).unwrap(), topic);
}
