use std::fs;
use std::path::PathBuf;

const BANNED: [&str; 8] = [
    "bus::classify",
    "classify_at",
    "gone_error",
    "read_failure",
    "CopyText",
    "CutText",
    "PasteText",
    "GrabFocus",
];

fn offences(source: &str) -> Vec<&'static str> {
    BANNED
        .iter()
        .copied()
        .filter(|banned| source.contains(banned))
        .collect()
}

fn shipped_sources() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/actions");
    let mut sources = Vec::new();
    for entry in fs::read_dir(dir).expect("the actions directory reads") {
        let path = entry.expect("an entry reads").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        if name.ends_with(".rs") && !name.ends_with("_tests.rs") {
            let text = fs::read_to_string(&path).expect("a source reads");
            sources.push((name, text));
        }
    }
    sources
}

#[test]
fn no_action_file_reaches_a_read_path_mapper_or_a_banned_call() {
    let sources = shipped_sources();
    assert!(sources.len() > 10, "the scan must see the action files");
    for (name, text) in sources {
        assert!(offences(&text).is_empty(), "{name}: {:?}", offences(&text));
    }
}
