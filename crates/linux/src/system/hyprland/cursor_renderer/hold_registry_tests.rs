use super::{acknowledged_renderers, ensure_held, is_held, mark_lost, publish, withdraw};

#[test]
fn a_published_hold_names_its_renderers_until_withdrawn() {
    publish("registry-a", vec![4_000_001, 4_000_002]);
    assert!(is_held("registry-a"));
    let pids = acknowledged_renderers();
    assert!(pids.contains(&4_000_001) && pids.contains(&4_000_002));
    withdraw("registry-a");
    assert!(!is_held("registry-a"));
    assert!(!acknowledged_renderers().contains(&4_000_001));
}

#[test]
fn a_lost_hold_refuses_the_next_event() {
    publish("registry-b", vec![]);
    mark_lost("registry-b");
    assert!(!is_held("registry-b"));
    let error = ensure_held().expect_err("a lost hold refuses");
    assert_eq!(
        error.details.expect("details")["kind"],
        "cursor_hide_renewal_failed"
    );
    withdraw("registry-b");
}

#[test]
fn another_threads_hold_is_invisible_to_this_one() {
    let other = std::thread::spawn(|| {
        publish("registry-c", vec![4_000_003]);
        mark_lost("registry-c");
    });
    other.join().expect("other thread");
    assert!(!acknowledged_renderers().contains(&4_000_003));
    assert!(ensure_held().is_ok());
    withdraw("registry-c");
}
