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

#[test]
fn an_app_that_answers_is_named_by_its_root() {
    let app = assemble(
        root(),
        facts(),
        Some(Some("gnome-calculator".to_owned())),
        ToolkitId::default(),
    );
    assert!(app.answered);
    assert_eq!(app.name, "gnome-calculator");
}

#[test]
fn an_app_that_answers_with_a_blank_name_is_answered_and_named_from_proc() {
    let app = assemble(root(), facts(), Some(None), ToolkitId::default());
    assert!(app.answered);
    assert_eq!(app.name, "gnome-calculator");
}

#[test]
fn a_skipped_record_names_the_bus_and_the_pid_when_known() {
    let record = skipped(":1.4", Some(7), "process is gone");
    assert_eq!(record["bus"], ":1.4");
    assert_eq!(record["pid"], 7);
    assert!(skipped(":1.4", None, "x").get("pid").is_none());
}
