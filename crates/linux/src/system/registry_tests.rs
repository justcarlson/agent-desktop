use std::cell::Cell;

use super::{AppRead, AppReads, read_known};
use crate::system::process_facts::ProcessFacts;
use crate::system::toolkit_id::ToolkitId;
use crate::tree::AtspiElement;

#[derive(Default)]
struct Counting {
    stopped: bool,
    remembered: Option<String>,
    saved: std::cell::RefCell<Vec<String>>,
    facts: Cell<u32>,
    names: Cell<u32>,
    toolkits: Cell<u32>,
}

impl AppReads for Counting {
    fn facts(&self, _pid: u32) -> Option<ProcessFacts> {
        self.facts.set(self.facts.get() + 1);
        Some(ProcessFacts {
            executable: None,
            comm: "x".to_owned(),
            exe_basename: Some("x".to_owned()),
            instance: "boot:1".to_owned(),
            web_family: None,
        })
    }

    fn is_stopped(&self, _pid: u32) -> bool {
        self.stopped
    }

    async fn name(&self, _root: &AtspiElement) -> Option<Option<String>> {
        self.names.set(self.names.get() + 1);
        Some(Some("App".to_owned()))
    }

    async fn toolkit(&self, _root: &AtspiElement) -> ToolkitId {
        self.toolkits.set(self.toolkits.get() + 1);
        ToolkitId::default()
    }

    fn remembered_name(&self, _root: &AtspiElement, _instance: &str) -> Option<String> {
        self.remembered.clone()
    }

    fn remember_name(&self, _root: &AtspiElement, _instance: &str, name: &str) {
        self.saved.borrow_mut().push(name.to_owned());
    }
}

fn root(pid: u32) -> AtspiElement {
    AtspiElement::new(":1.9", "/org/a11y/atspi/accessible/root", pid)
}

fn read(reads: &Counting, pid: u32, only: Option<u32>) -> AppRead {
    zbus::block_on(read_known(reads, root(pid), only))
}

#[test]
fn a_process_filter_never_reads_a_non_matching_pids_facts_name_or_toolkit() {
    let reads = Counting::default();
    assert!(matches!(read(&reads, 7, Some(8)), AppRead::Excluded));
    assert_eq!(
        (reads.facts.get(), reads.names.get(), reads.toolkits.get()),
        (0, 0, 0)
    );
}

#[test]
fn a_process_filter_reads_the_matching_pid_in_full() {
    let reads = Counting::default();
    assert!(matches!(read(&reads, 8, Some(8)), AppRead::Listed(_)));
    assert_eq!(
        (reads.facts.get(), reads.names.get(), reads.toolkits.get()),
        (1, 1, 1)
    );
}

#[test]
fn a_stopped_process_is_listed_unanswered_without_a_name_or_toolkit_read() {
    let reads = Counting {
        stopped: true,
        ..Counting::default()
    };
    let AppRead::Listed(app) = read(&reads, 7, None) else {
        panic!("a stopped app stays listed")
    };
    assert!(!app.answered);
    assert_eq!(reads.facts.get(), 1);
    assert_eq!((reads.names.get(), reads.toolkits.get()), (0, 0));
}

#[test]
fn a_running_process_is_read_for_its_name_and_toolkit() {
    let reads = Counting::default();
    let AppRead::Listed(app) = read(&reads, 7, None) else {
        panic!("listed")
    };
    assert!(app.answered);
    assert_eq!((reads.names.get(), reads.toolkits.get()), (1, 1));
    assert_eq!(reads.saved.borrow().as_slice(), ["App"]);
}

#[test]
fn a_stopped_process_keeps_its_registry_label_without_using_it_as_identity() {
    let reads = Counting {
        stopped: true,
        remembered: Some("QtApp".to_owned()),
        ..Counting::default()
    };
    let AppRead::Listed(app) = read(&reads, 7, None) else {
        panic!("a stopped app stays listed")
    };
    assert!(!app.answered);
    assert_eq!(app.registry_name.as_deref(), Some("QtApp"));
    assert!(!app.matches("qtapp"));
    assert!(app.matches("x"));
    assert!(!app.matches("unknown-app"));
    assert_eq!((reads.names.get(), reads.toolkits.get()), (0, 0));
}
