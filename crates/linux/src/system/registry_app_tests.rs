use super::*;

fn facts() -> ProcessFacts {
    ProcessFacts {
        comm: "gnome-calculato".to_owned(),
        exe_basename: Some("gnome-calculator".to_owned()),
        instance: "boot:42".to_owned(),
        web_family: None,
    }
}

fn root() -> AtspiElement {
    AtspiElement::new(":1.9", "/org/a11y/atspi/accessible/root", 321)
}

#[test]
fn a_stopped_app_that_never_answers_is_kept_and_named_from_proc() {
    let app = assemble(root(), facts(), None, ToolkitId::default());
    assert!(!app.answered);
    assert_eq!(app.name, "gnome-calculator");
    assert_eq!(app.root.pid(), 321);
    assert_eq!(app.facts.instance, "boot:42");
}

fn nautilus_facts() -> ProcessFacts {
    ProcessFacts {
        comm: "nautilus".to_owned(),
        exe_basename: Some("nautilus".to_owned()),
        instance: "boot:7".to_owned(),
        web_family: None,
    }
}

#[test]
fn the_listed_name_is_identical_whether_the_root_answers_or_the_app_is_stopped() {
    let answering = assemble(
        root(),
        nautilus_facts(),
        Some(Some("org.gnome.Nautilus".to_owned())),
        ToolkitId::default(),
    );
    let stopped = assemble(root(), nautilus_facts(), None, ToolkitId::default());
    assert!(answering.answered);
    assert!(!stopped.answered);
    assert_eq!(answering.name, "nautilus");
    assert_eq!(answering.name, stopped.name);
}

#[test]
fn the_registry_name_finds_an_answering_app_and_the_stable_name_finds_both() {
    let answering = assemble(
        root(),
        nautilus_facts(),
        Some(Some("org.gnome.Nautilus".to_owned())),
        ToolkitId::default(),
    );
    let stopped = assemble(root(), nautilus_facts(), None, ToolkitId::default());
    assert!(answering.matches("org.gnome.Nautilus"));
    assert!(!stopped.matches("org.gnome.Nautilus"));
    assert!(answering.matches("nautilus"));
    assert!(stopped.matches("nautilus"));
}

#[test]
fn an_app_that_answers_with_a_blank_name_has_no_registry_name() {
    let app = assemble(
        root(),
        facts(),
        Some(Some("  ".to_owned())),
        ToolkitId::default(),
    );
    assert!(app.answered);
    assert_eq!(app.registry_name, None);
    assert_eq!(app.name, "gnome-calculator");
}

#[test]
fn a_skipped_record_names_the_bus_and_the_pid_when_known() {
    let record = skipped(":1.4", Some(7), "process is gone");
    assert_eq!(record["bus"], ":1.4");
    assert_eq!(record["pid"], 7);
    assert!(skipped(":1.4", None, "x").get("pid").is_none());
}
