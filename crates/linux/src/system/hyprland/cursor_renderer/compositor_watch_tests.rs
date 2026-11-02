use super::reload_in;

#[test]
fn a_reload_event_is_found_among_other_events() {
    let mut partial = Vec::new();
    assert!(reload_in(
        &mut partial,
        b"activewindow>>foot,shell\nconfigreloaded>>\nworkspace>>2\n"
    ));
    assert!(partial.is_empty());
}

#[test]
fn a_reload_event_split_across_reads_is_found_once_its_line_ends() {
    let mut partial = Vec::new();
    assert!(!reload_in(&mut partial, b"openlayer>>x\nconfigrel"));
    assert!(!reload_in(&mut partial, b"oaded>>"));
    assert!(reload_in(&mut partial, b"\n"));
    assert!(!reload_in(&mut partial, b"workspace>>1\n"));
}

#[test]
fn other_events_and_titles_that_name_a_reload_are_not_reloads() {
    let mut partial = Vec::new();
    assert!(!reload_in(
        &mut partial,
        b"activewindow>>foot,configreloaded>>\nwindowtitle>>abc\n"
    ));
}

#[test]
fn an_overlong_line_is_dropped_without_reading_its_tail_as_an_event() {
    let mut partial = Vec::new();
    let mut long = b"windowtitlev2>>abc,".to_vec();
    long.extend(std::iter::repeat_n(b'x', 5000));
    assert!(!reload_in(&mut partial, &long));
    assert!(partial.len() <= 4096);
    assert!(!reload_in(&mut partial, b"configreloaded>>\n"));
    assert!(reload_in(&mut partial, b"configreloaded>>\n"));
}
