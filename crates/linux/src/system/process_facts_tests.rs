use super::{WebFamily, classify_web, parse_start_time, process_instance, read};

fn stat_with(comm: &str, start_time: u64) -> String {
    let after_state: Vec<String> = (4..=52)
        .map(|field| {
            if field == 22 {
                start_time.to_string()
            } else {
                "0".to_owned()
            }
        })
        .collect();
    format!("4242 ({comm}) S {}", after_state.join(" "))
}

#[test]
fn a_command_name_with_spaces_and_parentheses_does_not_shift_the_fields() {
    assert_eq!(
        parse_start_time(&stat_with("(a b) c", 987_654)),
        Some(987_654)
    );
}

#[test]
fn a_plain_stat_line_yields_its_start_time() {
    assert_eq!(parse_start_time(&stat_with("code", 31)), Some(31));
}

#[test]
fn a_line_with_no_closing_parenthesis_is_rejected() {
    assert_eq!(parse_start_time("4242 (broken S 1 2 3"), None);
}

#[test]
fn a_line_too_short_to_hold_the_start_time_is_rejected() {
    assert_eq!(parse_start_time("4242 (code) S 1 2 3"), None);
}

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
