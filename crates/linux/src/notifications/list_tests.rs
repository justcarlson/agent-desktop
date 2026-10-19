use super::*;
use crate::notifications::fake::{FakeWorld, log, reading, spec};
use agent_desktop_core::Deadline;

fn deadline() -> Deadline {
    Deadline::after(1_000).expect("a deadline")
}

fn world_of(specs: &[crate::notifications::fake::Spec]) -> FakeWorld {
    FakeWorld::new(&log(), vec![reading(specs, true)])
}

#[test]
fn identical_entries_keep_two_indices() {
    let world = world_of(&[spec("Mail", "Same"), spec("Mail", "Same")]);
    let listed = list_in(&world, &NotificationFilter::default(), deadline()).expect("listed");
    assert_eq!(
        listed.iter().map(|info| info.index).collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn a_filter_keeps_the_index_the_unfiltered_list_gave() {
    let world = world_of(&[spec("Mail", "a"), spec("Chat", "b"), spec("Files", "c")]);
    let filter = NotificationFilter {
        app: Some("chat".to_owned()),
        ..NotificationFilter::default()
    };
    let listed = list_in(&world, &filter, deadline()).expect("listed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].index, 2);
}

#[test]
fn the_limit_applies_after_the_filter() {
    let world = world_of(&[spec("Mail", "a"), spec("Chat", "b"), spec("Chat", "c")]);
    let filter = NotificationFilter {
        app: Some("chat".to_owned()),
        limit: Some(1),
        ..NotificationFilter::default()
    };
    let listed = list_in(&world, &filter, deadline()).expect("listed");
    assert_eq!(
        listed.iter().map(|info| info.index).collect::<Vec<_>>(),
        vec![2]
    );
}

#[test]
fn the_text_filter_matches_title_body_and_app() {
    let world = world_of(&[spec("Mail", "Invoice").body("due"), spec("Chat", "hi")]);
    let filter = NotificationFilter {
        text: Some("DUE".to_owned()),
        ..NotificationFilter::default()
    };
    let listed = list_in(&world, &filter, deadline()).expect("listed");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "Invoice");
}

#[test]
fn the_empty_landmark_is_an_empty_list() {
    let world = world_of(&[]);
    assert!(
        list_in(&world, &NotificationFilter::default(), deadline())
            .expect("listed")
            .is_empty()
    );
}

#[test]
fn a_container_with_neither_entries_nor_landmark_is_an_error() {
    let mut read = reading(&[], false);
    read.nodes.retain(|node| node.name != "No Notifications");
    let world = FakeWorld::new(&log(), vec![read]);
    let error = list_in(&world, &NotificationFilter::default(), deadline()).expect_err("error");
    assert_eq!(error.code, agent_desktop_core::ErrorCode::ElementNotFound);
}

#[test]
fn an_empty_list_without_the_landmark_text_is_empty_when_the_footer_button_is_there() {
    let mut read = reading(&[], true);
    read.nodes.retain(|node| node.name != "No Notifications");
    let world = FakeWorld::new(&log(), vec![read]);
    assert!(
        list_in(&world, &NotificationFilter::default(), deadline())
            .expect("listed")
            .is_empty()
    );
}

#[test]
fn a_cut_short_read_is_an_error_and_never_a_prefix() {
    let mut read = reading(&[spec("Mail", "a")], false);
    read.complete = false;
    let world = FakeWorld::new(&log(), vec![read]);
    assert!(list_in(&world, &NotificationFilter::default(), deadline()).is_err());
}

#[test]
fn only_the_message_list_container_is_read() {
    let log = log();
    let world = FakeWorld::new(&log, vec![reading(&[spec("Mail", "a")], false)]);
    list_in(&world, &NotificationFilter::default(), deadline()).expect("listed");
    assert!(log.borrow().iter().any(|line| line == "locate:MessageList"));
    assert!(
        !log.borrow()
            .iter()
            .any(|line| line.starts_with("click") || line == "open")
    );
}

#[test]
fn listing_json_carries_no_id_and_no_path_text() {
    let world = world_of(&[spec("Mail", "a").body("b").actions(&["Go"])]);
    let listed = list_in(&world, &NotificationFilter::default(), deadline()).expect("listed");
    let json = serde_json::to_value(&listed).expect("json");
    let keys: Vec<&String> = json[0].as_object().expect("object").keys().collect();
    assert!(
        keys.iter()
            .all(|key| ["index", "app_name", "title", "body", "actions"].contains(&key.as_str()))
    );
    assert!(!json.to_string().contains("/e1"));
}

#[test]
fn an_error_carries_no_id() {
    let mut read = reading(&[], false);
    read.nodes.retain(|node| node.name != "No Notifications");
    let world = FakeWorld::new(&log(), vec![read]);
    let error = list_in(&world, &NotificationFilter::default(), deadline()).expect_err("error");
    let text = format!("{error:?}").to_lowercase();
    assert!(!text.contains("\"id\"") && !text.contains("notification_id"));
}

#[test]
fn no_daemon_id_call_exists_in_the_notification_sources() {
    let forbidden = [
        concat!("Close", "Notification"),
        concat!("GetServer", "Information"),
    ];
    let directory = concat!(env!("CARGO_MANIFEST_DIR"), "/src/notifications");
    for entry in std::fs::read_dir(directory).expect("directory") {
        let path = entry.expect("entry").path();
        let source = std::fs::read_to_string(&path).expect("source");
        for word in forbidden {
            assert!(!source.contains(word), "{} contains {word}", path.display());
        }
    }
}
