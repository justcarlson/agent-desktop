use super::*;

fn methods(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn entry(enabled: bool, separator: bool) -> LayoutEntry {
    LayoutEntry {
        id: 1,
        label: "Open".into(),
        enabled,
        separator,
        submenu: false,
        checked: None,
        children: Vec::new(),
    }
}

#[test]
fn a_listed_activate_reports_click() {
    assert_eq!(
        actions(&methods(&["Activate", "SecondaryActivate"])),
        ["Click"]
    );
}

#[test]
fn secondary_methods_alone_report_no_click() {
    assert!(actions(&methods(&["SecondaryActivate"])).is_empty());
    assert!(actions(&methods(&["XAyatanaSecondaryActivate", "Scroll"])).is_empty());
}

#[test]
fn a_listed_context_menu_reports_right_click() {
    assert_eq!(actions(&methods(&["ContextMenu"])), ["RightClick"]);
    assert_eq!(
        actions(&methods(&["Activate", "ContextMenu"])),
        ["Click", "RightClick"]
    );
}

#[test]
fn an_entry_reports_click_enabled_or_not_and_a_separator_reports_none() {
    assert_eq!(entry_actions(&entry(true, false)), ["Click"]);
    assert_eq!(entry_actions(&entry(false, false)), ["Click"]);
    assert!(entry_actions(&entry(true, true)).is_empty());
}
