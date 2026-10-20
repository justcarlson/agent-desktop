use super::{ProcState, is_stopped, parse};

fn stat_with(comm: &str, state: &str, start_time: u64) -> String {
    let after_state: Vec<String> = (4..=52)
        .map(|field| {
            if field == 22 {
                start_time.to_string()
            } else {
                "0".to_owned()
            }
        })
        .collect();
    format!("4242 ({comm}) {state} {}", after_state.join(" "))
}

#[test]
fn a_command_name_with_spaces_and_parentheses_does_not_shift_the_fields() {
    let stat = parse(&stat_with("(a b) c", "S", 987_654)).expect("parses");
    assert_eq!(stat.start_time, 987_654);
    assert_eq!(stat.state, ProcState::Sleeping);
}

#[test]
fn a_command_name_that_looks_like_a_state_does_not_decide_the_state() {
    let stat = parse(&stat_with("a) T (b", "S", 5)).expect("parses");
    assert_eq!(stat.state, ProcState::Sleeping);
    assert_eq!(stat.start_time, 5);
}

#[test]
fn a_line_with_no_closing_parenthesis_is_rejected() {
    assert_eq!(parse("4242 (broken S 1 2 3"), None);
}

#[test]
fn a_line_with_the_parentheses_reversed_is_rejected() {
    assert_eq!(
        parse("4242 )broken( S 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0"),
        None
    );
}

#[test]
fn a_line_too_short_to_hold_the_start_time_is_rejected() {
    assert_eq!(parse("4242 (code) S 1 2 3"), None);
}

#[test]
fn every_state_letter_maps_to_its_class() {
    let table = [
        ("R", ProcState::Running),
        ("S", ProcState::Sleeping),
        ("D", ProcState::DiskSleep),
        ("T", ProcState::Stopped),
        ("t", ProcState::Stopped),
        ("Z", ProcState::Zombie),
        ("X", ProcState::Dead),
        ("I", ProcState::Idle),
    ];
    for (letter, class) in table {
        let stat = parse(&stat_with("x", letter, 1)).expect("parses");
        assert_eq!(stat.state, class, "{letter}");
    }
}

#[test]
fn an_unknown_state_letter_is_other_and_not_a_known_class() {
    for letter in ["Q", "K", "W"] {
        let stat = parse(&stat_with("x", letter, 1)).expect("parses");
        assert_eq!(stat.state, ProcState::Other, "{letter}");
    }
}

#[test]
fn a_process_nothing_runs_under_is_not_stopped() {
    assert!(!is_stopped(u32::MAX - 1));
}
