use super::WindowIds;

fn store(name: &str) -> (WindowIds, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("ad-ids-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    (WindowIds::at(dir.clone()), dir)
}

#[test]
fn a_remembered_id_comes_back_for_the_same_client_and_instance_only() {
    let (ids, dir) = store("round");
    ids.remember(0x40_0003, "boot:1", "w-1.3-a");
    assert_eq!(ids.recall(0x40_0003, "boot:1").as_deref(), Some("w-1.3-a"));
    assert_eq!(ids.recall(0x40_0003, "boot:2"), None);
    assert_eq!(ids.recall(0x40_0004, "boot:1"), None);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_new_id_for_a_reused_client_replaces_the_old_one() {
    let (ids, dir) = store("replace");
    ids.remember(7, "boot:1", "w-1.3-a");
    ids.remember(7, "boot:1", "w-1.3-b");
    assert_eq!(ids.recall(7, "boot:1").as_deref(), Some("w-1.3-b"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn nothing_is_left_pending_and_the_directory_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let (ids, dir) = store("private");
    ids.remember(7, "boot:1", "w-1.3-a");
    let names: Vec<_> = std::fs::read_dir(&dir)
        .expect("the directory")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.iter().all(|name| !name.ends_with(".tmp")),
        "{names:?}"
    );
    let mode = std::fs::metadata(&dir)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o700);
    let _ = std::fs::remove_dir_all(dir);
}
