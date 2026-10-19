use std::cell::Cell;

use super::{AppRead, AppReads, read_known};
use crate::system::process_facts::ProcessFacts;
use crate::system::toolkit_id::ToolkitId;
use crate::tree::AtspiElement;

#[derive(Default)]
struct Counting {
    facts: Cell<u32>,
    names: Cell<u32>,
    toolkits: Cell<u32>,
}

impl AppReads for Counting {
    fn facts(&self, _pid: u32) -> Option<ProcessFacts> {
        self.facts.set(self.facts.get() + 1);
        Some(ProcessFacts {
            comm: "x".to_owned(),
            exe_basename: Some("x".to_owned()),
            instance: "boot:1".to_owned(),
            web_family: None,
        })
    }

    async fn name(&self, _root: &AtspiElement) -> Option<Option<String>> {
        self.names.set(self.names.get() + 1);
        Some(Some("App".to_owned()))
    }

    async fn toolkit(&self, _root: &AtspiElement) -> ToolkitId {
        self.toolkits.set(self.toolkits.get() + 1);
        ToolkitId::default()
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
fn no_filter_reads_every_pid() {
    let reads = Counting::default();
    assert!(matches!(read(&reads, 7, None), AppRead::Listed(_)));
    assert!(matches!(read(&reads, 9, None), AppRead::Listed(_)));
    assert_eq!(reads.names.get(), 2);
}
