use agent_desktop_core::{LocatorEvidence, LocatorField, Rect, state};

use super::super::menu_layout::{Prop, RawEntry, parse};
use super::super::{TrayTarget, identity};
use super::*;

fn target(owner: &str) -> TrayTarget {
    TrayTarget {
        owner: owner.into(),
        object_path: "/StatusNotifierItem".into(),
        menu_path: None,
        entry_id: None,
        revision: None,
        label_path: Vec::new(),
    }
}

fn props(title: &str, passive: bool, menu: Option<&str>) -> ItemProps {
    ItemProps {
        title: title.into(),
        id: title.to_lowercase(),
        passive,
        menu: menu.map(str::to_owned),
        item_is_menu: false,
    }
}

fn entry(id: i32, label: &str) -> RawEntry {
    RawEntry {
        id,
        props: [("label".to_owned(), Prop::Text(label.into()))]
            .into_iter()
            .collect(),
        children: Vec::new(),
    }
}

fn item(owner: &str, title: &str, passive: bool, methods: &[&str], with_menu: bool) -> TrayItem {
    let layout = with_menu.then(|| {
        parse(
            3,
            &RawEntry {
                id: 0,
                props: Default::default(),
                children: vec![entry(1, "Open"), entry(2, "Quit")],
            },
        )
    });
    TrayItem {
        target: target(owner),
        props: props(title, passive, with_menu.then_some("/MenuBar")),
        methods: methods.iter().map(|m| (*m).to_owned()).collect(),
        layout,
    }
}

fn view() -> TrayView {
    TrayView {
        items: vec![
            item(":1.1", "Dropbox", false, &["Activate", "ContextMenu"], true),
            item(":1.2", "Twin", false, &["Activate"], false),
            item(":1.3", "Twin", false, &["SecondaryActivate"], false),
            item(":1.4", "Slack", true, &[], false),
        ],
    }
}

fn panel_evidence(rect: Rect) -> LocatorEvidence {
    let mut evidence = fresh(
        "menu",
        "Dropbox",
        &target(":9.9"),
        vec!["showing".into()],
        vec![],
    );
    evidence.description = LocatorField::Known("panel text".into());
    evidence.value = LocatorField::Known("panel value".into());
    evidence.ref_evidence.bounds = LocatorField::Known(rect);
    evidence
}

fn titles(strings: &[&str]) -> Vec<String> {
    strings.iter().map(|title| (*title).to_owned()).collect()
}

fn decoded(evidence: &LocatorEvidence) -> TrayTarget {
    let value = evidence
        .identifiers
        .preferred_value()
        .expect("an identifier is set");
    identity::decode(value).expect("it decodes")
}

#[test]
fn unique_titles_bind_and_duplicates_bind_to_nothing() {
    let bound = view().bind(&titles(&["Slack", "Twin", "Dropbox", "AnyDesk"]));
    assert_eq!(bound, vec![Some(3), None, Some(0), None]);
}

#[test]
fn a_duplicate_title_item_stays_reachable_as_a_node_of_its_own() {
    let view = view();
    let bound = view.bind(&titles(&["Slack", "Twin", "Dropbox"]));
    let extra = view.unpaired(&bound);
    assert_eq!(extra.len(), 2);
}

#[test]
fn a_bound_item_is_not_listed_a_second_time() {
    let view = view();
    let bound = view.bind(&titles(&["Dropbox"]));
    assert_eq!(view.unpaired(&bound).len(), 3);
}

#[test]
fn decorating_a_panel_node_gives_it_the_items_identity_and_clears_panel_text() {
    let view = view();
    let mut evidence = panel_evidence(Rect {
        x: 10.0,
        y: 0.0,
        width: 48.0,
        height: 32.0,
    });
    view.decorate(0, &mut evidence, 0);
    assert_eq!(decoded(&evidence), target(":1.1"));
    assert_eq!(evidence.role, LocatorField::Known("menubutton".into()));
    assert_eq!(evidence.description, LocatorField::Absent);
    assert_eq!(evidence.value, LocatorField::Absent);
    assert_eq!(
        evidence.ref_evidence.bounds,
        LocatorField::Known(Rect {
            x: 10.0,
            y: 0.0,
            width: 48.0,
            height: 32.0
        })
    );
}

#[test]
fn a_passive_item_is_hidden_while_its_node_keeps_valid_half_height_extents() {
    let view = view();
    let mut evidence = panel_evidence(Rect {
        x: 1094.0,
        y: 0.0,
        width: 48.0,
        height: 16.0,
    });
    view.decorate(3, &mut evidence, 0);
    let states = evidence.states.known().expect("states known");
    assert!(state::has_state(states, state::HIDDEN), "{states:?}");
}

#[test]
fn an_active_item_with_a_collapsed_node_is_not_hidden() {
    let view = view();
    let mut evidence = panel_evidence(Rect {
        x: -1.0,
        y: -1.0,
        width: 0.0,
        height: 0.0,
    });
    view.decorate(0, &mut evidence, 0);
    let states = evidence.states.known().expect("states known");
    assert!(!state::has_state(states, state::HIDDEN), "{states:?}");
}

#[test]
fn capabilities_follow_the_listed_methods() {
    let view = view();
    for (index, expected) in [
        (0, vec!["Click", "RightClick"]),
        (1, vec!["Click"]),
        (2, vec![]),
        (3, vec![]),
    ] {
        let mut evidence = panel_evidence(Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        });
        view.decorate(index, &mut evidence, 0);
        assert_eq!(
            evidence.ref_evidence.available_actions,
            LocatorField::Known(expected.iter().map(|a| (*a).to_owned()).collect()),
            "item {index}"
        );
    }
}

#[test]
fn an_item_with_a_menu_yields_its_entries_numbered_from_the_first_free_index() {
    let view = view();
    let mut evidence = panel_evidence(Rect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    let children = view.decorate(0, &mut evidence, 5);
    assert_eq!(children.len(), 2);
}

#[test]
fn an_item_with_no_menu_yields_no_children() {
    let view = view();
    let mut evidence = panel_evidence(Rect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    assert!(view.decorate(1, &mut evidence, 0).is_empty());
}

#[test]
fn an_unbound_panel_node_is_marked() {
    let mut evidence = panel_evidence(Rect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    TrayView::mark_unbound(&mut evidence);
    assert_eq!(
        evidence.description,
        LocatorField::Known("tray item without D-Bus identity".into())
    );
}
