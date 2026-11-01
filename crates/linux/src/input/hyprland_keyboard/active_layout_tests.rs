use agent_desktop_core::DeliverySemantics;
use serde_json::json;

use super::active_layout;

fn layouts(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn devices(keyboards: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&json!({ "mice": [], "keyboards": keyboards })).expect("serializes")
}

fn refused_cause(error: &agent_desktop_core::AdapterError) -> Option<String> {
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    error
        .details
        .as_ref()
        .and_then(|details| details["cause"].as_str())
        .map(str::to_owned)
}

#[test]
fn the_main_keyboard_index_names_the_layout_over_any_other_keyboard() {
    let reply = devices(json!([
        { "name": "virtual", "main": false, "active_keymap": "English (US)", "active_layout_index": 0 },
        { "name": "at-kbd", "main": true, "active_keymap": "Russian", "active_layout_index": 1 },
    ]));
    let found = active_layout(&reply, &layouts(&["English (US)", "Russian"]));
    assert_eq!(found.expect("the layout reads"), 1);
}

#[test]
fn without_an_index_the_active_keymap_name_picks_the_layout() {
    let reply = devices(json!([{ "name": "at-kbd", "main": true, "active_keymap": "Russian" }]));
    let found = active_layout(&reply, &layouts(&["English (US)", "Russian"]));
    assert_eq!(found.expect("the layout reads"), 1);
}

#[test]
fn a_single_layout_keymap_needs_no_answer() {
    for reply in [devices(json!([])), b"not json".to_vec()] {
        let found = active_layout(&reply, &layouts(&["German"]));
        assert_eq!(found.expect("the only layout"), 0);
    }
}

#[test]
fn an_unproven_layout_of_a_multi_layout_keymap_is_refused() {
    let two = layouts(&["English (US)", "English (US)"]);
    for (reply, names) in [
        (devices(json!([])), layouts(&["English (US)", "Russian"])),
        (
            devices(json!([{ "main": true, "active_keymap": "English (US)" }])),
            two.clone(),
        ),
        (
            devices(json!([{ "main": true, "active_keymap": "Greek" }])),
            layouts(&["English (US)", "Russian"]),
        ),
        (
            devices(json!([{ "main": true, "active_layout_index": 2 }])),
            layouts(&["English (US)", "Russian"]),
        ),
    ] {
        let error = active_layout(&reply, &names).expect_err("the layout is unproven");
        assert_eq!(
            refused_cause(&error).as_deref(),
            Some("active_layout_unknown")
        );
    }
}
