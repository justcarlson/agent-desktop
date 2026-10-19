use std::collections::HashSet;

use agent_desktop_core::RefEntry;

/// The refs of a window that a survey leaves unjudged, by id. Files lists the
/// mounts of the session in its places sidebar, so that list and every row
/// and button below it come and go while a survey runs, which is not a
/// disagreement between two reads of one node.
pub(super) fn unjudged(label: &str, entries: &[(String, RefEntry)]) -> HashSet<String> {
    if label != "files" {
        return HashSet::new();
    }
    let roots: Vec<_> = entries
        .iter()
        .filter(|(_, entry)| entry.identity.role == "listbox")
        .map(|(_, entry)| entry.scope.path.clone())
        .collect();
    let skipped: HashSet<String> = entries
        .iter()
        .filter(|(_, entry)| {
            roots
                .iter()
                .any(|root| entry.scope.path.starts_with(root.as_slice()))
        })
        .map(|(id, _)| id.clone())
        .collect();
    println!(
        "live survey {label}: {} refs under the places list not judged, its mount rows churn",
        skipped.len()
    );
    skipped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::test_support::ref_entry;

    fn entry_at(role: &str, path: &[usize]) -> RefEntry {
        let mut entry = ref_entry(role);
        entry.scope.path = path.iter().copied().collect();
        entry
    }

    #[test]
    fn only_files_leaves_refs_under_a_listbox_unjudged() {
        let entries = vec![
            ("@e1".to_owned(), entry_at("listbox", &[0, 1])),
            ("@e2".to_owned(), entry_at("button", &[0, 1, 0])),
            ("@e3".to_owned(), entry_at("button", &[0, 2])),
        ];
        let skipped = unjudged("files", &entries);
        assert!(skipped.contains("@e1") && skipped.contains("@e2"));
        assert!(!skipped.contains("@e3"));
        assert!(unjudged("calculator", &entries).is_empty());
    }
}
