use super::*;

#[test]
fn states_follow_enabled_checked_and_submenu() {
    let mut entry = LayoutEntry {
        id: 4,
        label: "Check".into(),
        enabled: true,
        separator: false,
        submenu: false,
        checked: None,
        children: Vec::new(),
    };
    assert_eq!(states_of(&entry), ["offscreen"]);
    entry.enabled = false;
    entry.checked = Some(true);
    entry.submenu = true;
    assert_eq!(
        states_of(&entry),
        ["offscreen", "disabled", "checked", "haspopup"]
    );
}
