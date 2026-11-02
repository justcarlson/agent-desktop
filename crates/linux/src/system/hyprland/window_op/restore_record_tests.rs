use super::*;

#[test]
fn a_record_round_trips_through_its_tag_with_negative_offsets() {
    let record = RestoreRecord {
        offset: (-50, -20),
        size: (500, 400),
    };

    assert_eq!(record.tag(), "agent-desktop-restore:-50,-20,500,400");
    assert_eq!(RestoreRecord::parse(&record.tag()), Some(record));
}

#[test]
fn a_tag_that_is_not_a_whole_record_is_no_record() {
    for tag in [
        "agent-desktop-restore:1,2,3",
        "agent-desktop-restore:1,2,0,4",
        "agent-desktop-restore:1,2,3,x",
        "agent-desktop-restore:1,2,3,4,5",
        "work",
    ] {
        assert_eq!(RestoreRecord::parse(tag), None, "{tag}");
    }
}

#[test]
fn only_restore_tags_are_picked_from_a_window_s_tags() {
    let tags = vec![
        "work".to_owned(),
        "agent-desktop-restore:1,2,3,4".to_owned(),
        "agent-desktop-restore:bad".to_owned(),
    ];

    assert_eq!(
        restore_tags(&tags),
        vec!["agent-desktop-restore:1,2,3,4", "agent-desktop-restore:bad"]
    );
}
