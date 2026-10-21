use agent_desktop_core::{
    Action, ActionOps, ActionRequest, DeliveryDisposition, ErrorCode, ObservationOps,
};

use super::live_rig::{Rig, deadline, lease};

fn rig_with_two_items(name: &str) -> Rig {
    let mut rig = Rig::new(name);
    rig.spawn(&["watcher", "watcher"]);
    rig.spawn(&["kde", "KdeItem", "Dropbox", "menu"]);
    rig.spawn(&["ayatana", "AyaItem", "Slack", "menu"]);
    rig
}

fn item<'a>(view: &'a super::view::TrayView, title: &str) -> &'a super::view::TrayItem {
    view.items
        .iter()
        .find(|item| item.props.title == title)
        .unwrap_or_else(|| panic!("the tray lists '{title}'"))
}

#[test]
#[ignore = "needs a private session bus; run through scripts/private-session.sh"]
fn tray_live_lists_registered_items_with_identity_methods_and_menu() {
    let rig = rig_with_two_items("list");
    let view = rig.view();
    assert_eq!(view.items.len(), 2, "{:?}", rig.log());
    let kde = item(&view, "Dropbox");
    assert!(kde.target.owner.starts_with(':'), "{:?}", kde.target);
    assert_eq!(kde.target.object_path, "/StatusNotifierItem");
    assert_eq!(
        kde.methods,
        ["Activate", "ContextMenu", "SecondaryActivate", "Scroll"]
    );
    let layout = kde.layout.as_ref().expect("the menu layout parsed");
    let labels: Vec<&str> = layout
        .root
        .children
        .iter()
        .map(|e| e.label.as_str())
        .collect();
    assert_eq!(labels, ["Open L32", "", "Disabled", "Check", "More"]);
    assert!(layout.root.children[1].separator);
    assert!(!layout.root.children[2].enabled);
    assert_eq!(layout.root.children[3].checked, Some(false));
    assert_eq!(layout.root.children[4].children.len(), 1);
    let aya = item(&view, "Slack");
    assert_eq!(
        aya.methods,
        ["Scroll", "SecondaryActivate", "XAyatanaSecondaryActivate"]
    );
    assert!(
        aya.target
            .object_path
            .starts_with("/org/ayatana/NotificationItem/")
    );
}

#[test]
#[ignore = "needs a private session bus; run through scripts/private-session.sh"]
fn tray_live_marks_passive_from_status_and_not_from_geometry() {
    let mut rig = rig_with_two_items("passive");
    assert!(!item(&rig.view(), "Slack").props.passive);
    rig.signal(2);
    rig.wait_for("status AyaItem Passive");
    assert!(item(&rig.view(), "Slack").props.passive);
    rig.signal(2);
    rig.wait_for("status AyaItem Active");
    assert!(!item(&rig.view(), "Slack").props.passive);
    rig.stop(1);
}

#[test]
#[ignore = "needs a private session bus; run through scripts/private-session.sh"]
fn tray_live_context_menu_uses_the_primary_context_method() {
    let rig = rig_with_two_items("context-menu");
    let view = rig.view();
    let kde = rig.item_entry(item(&view, "Dropbox"));
    let handle = rig.resolve(&kde).expect("the KDE-style item resolves");
    rig.adapter
        .execute_action(
            &handle,
            ActionRequest::headless(Action::RightClick),
            &lease(),
        )
        .expect("ContextMenu is delivered");
    rig.wait_for("item KdeItem method ContextMenu 0 0");
}

#[test]
#[ignore = "needs a private session bus; run through scripts/private-session.sh"]
fn tray_live_menu_pick_validates_then_sends_the_event() {
    let rig = rig_with_two_items("menu");
    let view = rig.view();
    let kde = item(&view, "Dropbox");
    let pick = |id, labels: &[&str]| {
        let handle = rig.resolve(&rig.menu_entry(kde, id, labels))?;
        rig.adapter
            .execute_action(&handle, ActionRequest::headless(Action::Click), &lease())
    };

    pick(1, &["Open L32"]).expect("an enabled entry is picked");
    rig.wait_for("menu KdeItem Event 1 clicked");

    let error = pick(3, &["Disabled"]).expect_err("a disabled entry is refused");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    let error = pick(2, &[""]).expect_err("a separator is refused");
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert_eq!(rig.count("Event 3"), 0);
    assert_eq!(rig.count("Event 2"), 0);

    let result = pick(4, &["Check"]).expect("a checkable entry is picked");
    assert_eq!(
        result.disposition().delivery(),
        DeliveryDisposition::DeliveredVerified
    );
    rig.wait_for("menu KdeItem Event 4 clicked");

    pick(5, &["More"]).expect("a submenu parent is opened");
    rig.wait_for("menu KdeItem AboutToShow 5");
    assert_eq!(rig.count("Event 5"), 0);

    pick(6, &["More", "Child"]).expect("a nested entry is picked");
    rig.wait_for("menu KdeItem Event 6 clicked");
}

#[test]
#[ignore = "needs a private session bus; run through scripts/private-session.sh"]
fn tray_live_a_restarted_owner_is_a_stale_ref_and_never_reached() {
    let mut rig = rig_with_two_items("churn");
    let view = rig.view();
    let old = rig.item_entry(item(&view, "Dropbox"));
    let old_owner = item(&view, "Dropbox").target.owner.clone();
    rig.resolve(&old)
        .expect("the item resolves before it restarts");
    rig.stop(1);
    rig.spawn(&["kde", "KdeItem", "Dropbox", "menu"]);
    let activations = rig.count("method Activate");
    let error = rig
        .resolve(&old)
        .err()
        .expect("the old unique name is gone");
    assert_eq!(error.code, ErrorCode::StaleRef);
    let fresh = rig.view();
    let new_owner = item(&fresh, "Dropbox").target.owner.clone();
    assert_ne!(new_owner, old_owner);
    assert_eq!(rig.count("method Activate"), activations);
}

#[test]
#[ignore = "needs a private session bus; run through scripts/private-session.sh"]
fn tray_live_an_item_that_exits_drops_out_of_the_list() {
    let mut rig = rig_with_two_items("exit");
    assert_eq!(rig.view().items.len(), 2);
    rig.stop(2);
    let started = std::time::Instant::now();
    while rig.view().items.len() != 1 {
        assert!(started.elapsed().as_secs() < 10, "{:?}", rig.log());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert_eq!(rig.view().items[0].props.title, "Dropbox");
}
