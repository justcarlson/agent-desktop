use super::{Lookup, WebFamily, classify_web, lookup, lookup_for_error, process_instance, read};

#[test]
fn different_start_times_give_different_tokens() {
    assert_ne!(
        process_instance("0123456789abcdef", 10),
        process_instance("0123456789abcdef", 11)
    );
}

#[test]
fn a_different_boot_gives_a_different_token_for_the_same_start_time() {
    assert_ne!(
        process_instance("aaaaaaaa-1111", 10),
        process_instance("bbbbbbbb-1111", 10)
    );
}

#[test]
fn the_token_is_never_empty() {
    assert!(!process_instance("", 0).is_empty());
}

#[test]
fn the_known_browser_binaries_classify_to_their_family() {
    for name in ["code", "chrome", "chromium", "electron", "google-chrome"] {
        assert_eq!(
            classify_web(name, None, false),
            Some(WebFamily::Chromium),
            "{name}"
        );
    }
    assert_eq!(classify_web("firefox", None, false), Some(WebFamily::Gecko));
    assert_eq!(
        classify_web("x", Some("firefox-bin"), false),
        Some(WebFamily::Gecko)
    );
}

#[test]
fn the_exe_basename_classifies_when_comm_was_truncated_or_renamed() {
    assert_eq!(
        classify_web("Isolated Web Co", Some("Chrome"), false),
        Some(WebFamily::Chromium)
    );
}

#[test]
fn an_unlisted_binary_with_a_chromium_payload_beside_it_is_chromium() {
    assert_eq!(
        classify_web("slack", Some("slack"), true),
        Some(WebFamily::Chromium)
    );
}

#[test]
fn a_native_application_is_in_no_family() {
    assert_eq!(classify_web("nautilus", Some("nautilus"), false), None);
}

#[test]
fn a_process_that_does_not_exist_has_no_facts() {
    assert_eq!(read(u32::MAX), None);
}

#[cfg(target_os = "linux")]
#[test]
fn the_current_process_reads_a_stable_token() {
    let first = read(std::process::id()).expect("the test process is running");
    let second = read(std::process::id()).expect("the test process is running");
    assert_eq!(first.instance, second.instance);
    assert!(!first.comm.is_empty());
}

#[test]
fn only_a_read_that_failed_because_the_process_left_says_gone() {
    use std::io::{Error, ErrorKind};
    assert_eq!(
        lookup_for_error(&Error::from(ErrorKind::NotFound)),
        Lookup::Gone
    );
    assert_eq!(lookup_for_error(&Error::from_raw_os_error(3)), Lookup::Gone);
    assert_eq!(
        lookup_for_error(&Error::from(ErrorKind::PermissionDenied)),
        Lookup::Unreadable
    );
    assert_eq!(
        lookup_for_error(&Error::from_raw_os_error(24)),
        Lookup::Unreadable
    );
}

#[test]
fn a_process_id_nothing_runs_under_is_gone() {
    assert_eq!(lookup(u32::MAX - 1), Lookup::Gone);
}

#[test]
#[cfg(target_os = "linux")]
fn this_process_is_found() {
    assert!(matches!(lookup(std::process::id()), Lookup::Found(_)));
}
