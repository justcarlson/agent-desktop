use super::disclosure::set_expanded;
use super::live_click_tests::{act, find, open, state};
use crate::tree::live_launch::nonce;
use crate::tree::state_bits::{EXPANDABLE, EXPANDED};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

struct Fixture {
    directory: PathBuf,
    settings: Vec<(&'static str, &'static str, String)>,
}

fn setting(operation: &str, schema: &str, key: &str, value: Option<&str>) -> String {
    let output = Command::new("gsettings")
        .args([operation, schema, key])
        .args(value)
        .output()
        .expect("gsettings runs");
    assert!(output.status.success(), "gsettings: {:?}", output.stderr);
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

impl Fixture {
    fn new() -> Self {
        assert!(std::env::var_os("AGENT_DESKTOP_PRIVATE_SESSION").is_some());
        let mut fixture = Self {
            directory: std::env::temp_dir().join(nonce("disclosure")),
            settings: Vec::new(),
        };
        for (schema, key, value) in [
            ("org.gnome.nautilus.list-view", "use-tree-view", "true"),
            (
                "org.gnome.nautilus.preferences",
                "default-folder-viewer",
                "'list-view'",
            ),
        ] {
            fixture
                .settings
                .push((schema, key, setting("get", schema, key, None)));
            setting("set", schema, key, Some(value));
        }
        std::fs::create_dir_all(fixture.directory.join("folder")).unwrap();
        std::fs::write(fixture.directory.join("folder/leaf.txt"), "tree leaf").unwrap();
        fixture
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for (schema, key, value) in &self.settings {
            setting("set", schema, key, Some(value));
        }
        std::fs::remove_dir_all(&self.directory).expect("fixture removed");
    }
}

fn oracle(expected: (bool, bool)) {
    let started = Instant::now();
    loop {
        let output = Command::new("python3")
            .args([
                "-c",
                r#"
import gi, json
gi.require_version('Atspi', '2.0')
from gi.repository import Atspi
pending = [(Atspi.get_desktop(0), False)]
rows, leaves = [], []
while pending:
    node, in_folder = pending.pop()
    in_folder = in_folder or (node.get_role() == Atspi.Role.TABLE_ROW and node.get_name() == 'folder')
    states = node.get_state_set()
    if in_folder and states.contains(Atspi.StateType.EXPANDABLE):
        rows.append(states.contains(Atspi.StateType.EXPANDED))
    if node.get_name() == 'leaf.txt' and states.contains(Atspi.StateType.SHOWING):
        leaves.append(node)
    pending.extend((node.get_child_at_index(i), in_folder) for i in range(node.get_child_count()))
assert len(rows) == 1, rows
print(json.dumps([rows[0], bool(leaves)]))
"#,
            ])
            .output()
            .expect("independent AT-SPI oracle runs");
        assert!(output.status.success(), "oracle: {:?}", output.stderr);
        let observed: (bool, bool) = serde_json::from_slice(&output.stdout).expect("oracle states");
        if observed == expected {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "oracle: {observed:?}, expected {expected:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[ignore = "live: needs the AT-SPI session, Files (nautilus) with a folder tree and the write engine"]
fn a_files_tree_row_expands_and_collapses_in_both_directions() {
    let fixture = Fixture::new();
    let session = open(
        "nautilus",
        &["--new-window", fixture.directory.to_str().unwrap()],
    )
    .expect("files opens");
    let row = find(&session, |view| {
        view.state(EXPANDABLE) == Some(true) && view.state(EXPANDED) == Some(false)
    })
    .expect("fixture has a collapsed expandable folder control");
    assert_eq!(state(&session, &row, EXPANDED), Some(false));
    oracle((false, false));
    let opened = act(&session, &row, |target| set_expanded(target, true)).expect("expands");
    assert_eq!(opened.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &row, EXPANDED), Some(true));
    oracle((true, true));
    let leaf = find(&session, |view| view.name() == Some("leaf.txt")).expect("expanded leaf");
    let refused = act(&session, &leaf, |target| set_expanded(target, true))
        .expect_err("a file cannot expand");
    assert_eq!(
        refused.code,
        agent_desktop_core::ErrorCode::ActionNotSupported
    );
    oracle((true, true));
    let closed = act(&session, &row, |target| set_expanded(target, false)).expect("collapses");
    assert_eq!(closed.steps[0].verified(), Some(true));
    assert_eq!(state(&session, &row, EXPANDED), Some(false));
    oracle((false, false));
}
