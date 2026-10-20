use super::*;

fn app(pid: u32, name: &str, comm: &str) -> Running {
    Running {
        pid,
        instance: format!("boot:{pid}"),
        name: name.to_owned(),
        registry_name: None,
        comm: comm.to_owned(),
    }
}

fn keys(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn no_match_spawns() {
    let running = [app(1, "nautilus", "nautilus")];
    let decision = decide(&keys(&["gnome-calculator"]), &running, true, false);
    assert_eq!(decision, Decision::Spawn);
}

#[test]
fn one_match_without_arguments_attaches_and_spawns_nothing() {
    let owner = app(7, "gnome-text-editor", "gnome-text-edit");
    let decision = decide(
        &keys(&["gnome-text-editor"]),
        std::slice::from_ref(&owner),
        true,
        false,
    );
    assert_eq!(decision, Decision::Attach(owner));
}

#[test]
fn one_match_with_arguments_spawns_and_reports_the_owner() {
    let owner = app(7, "gnome-text-editor", "gnome-text-edit");
    let decision = decide(
        &keys(&["gnome-text-editor"]),
        std::slice::from_ref(&owner),
        true,
        true,
    );
    assert_eq!(decision, Decision::Forward(owner));
}

#[test]
fn a_match_with_attach_off_refuses_naming_the_pid() {
    let owner = app(7, "gnome-calculator", "gnome-calculato");
    let decision = decide(&keys(&["gnome-calculator"]), &[owner], false, false);
    assert_eq!(decision, Decision::Refuse(vec![7]));
}

#[test]
fn two_matches_are_ambiguous_and_two_with_attach_off_still_refuse() {
    let running = [
        app(7, "nautilus", "nautilus"),
        app(8, "nautilus", "nautilus"),
    ];
    let wanted = keys(&["nautilus"]);
    assert_eq!(
        decide(&wanted, &running, true, false),
        Decision::Ambiguous(vec![7, 8])
    );
    assert_eq!(
        decide(&wanted, &running, false, false),
        Decision::Refuse(vec![7, 8])
    );
}

#[test]
fn a_key_matches_the_name_the_registry_name_or_the_cut_comm() {
    let mut owner = app(1, "gnome-calculator", "gnome-calculato");
    owner.registry_name = Some("Calculator".to_owned());
    assert!(matches(&keys(&["gnome-calculator"]), &owner));
    assert!(matches(&keys(&["calculator"]), &owner));
    assert!(matches(&keys(&["gnome-calculato"]), &owner));
    let renamed = app(2, "wrapper", "gnome-calculato");
    assert!(matches(&keys(&["gnome-calculator"]), &renamed));
}

#[test]
fn a_substring_never_matches_and_the_launcher_is_not_the_server() {
    let server = app(1, "gnome-terminal-server", "gnome-terminal-");
    assert!(!matches(&keys(&["gnome-terminal"]), &server));
    assert!(!matches(&keys(&["terminal"]), &server));
    assert!(!matches(&keys(&["gnome-termina"]), &server));
    let short = app(2, "tool", "tool");
    assert!(!matches(&keys(&["tool-extra"]), &short));
}

#[test]
fn the_terminal_keys_include_the_server_exactly() {
    let table = identity_keys(&keys(&[
        "org.gnome.Terminal",
        "gnome-terminal",
        "GNOME-Terminal",
    ]));
    assert_eq!(
        table,
        [
            "org.gnome.terminal",
            "gnome-terminal-server",
            "gnome-terminal"
        ]
    );
    let server = app(1, "gnome-terminal-server", "gnome-terminal-");
    assert!(matches(&table, &server));
}

#[test]
fn keys_are_lower_cased_and_unique() {
    assert_eq!(identity_keys(&keys(&["Tool", "tool", "TOOL"])), ["tool"]);
}
