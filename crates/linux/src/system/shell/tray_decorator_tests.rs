use std::cell::RefCell;
use std::rc::Rc;

use agent_desktop_core::{
    AccessibilityNode, LocatorEvidence, LocatorField, ObservationSource, ObservedSubtree,
    ObservedTree, WindowInfo, WindowState,
};

use crate::system::shell::container::MENU;
use crate::system::shell::tray_decorator::{Items, TrayDecorator};
use crate::tree::decorate::Decorate;
use crate::tree::projection::{project_node, project_with};
use crate::tree::role_numbers::{PANEL, PUSH_BUTTON};
use crate::tree::test_support::read_of;
use crate::tree::{ObservedRead, Walk, WalkCounts, WalkLimits, WalkedNode};

struct Fake {
    titles: Vec<&'static str>,
    log: Rc<RefCell<Vec<String>>>,
}

fn entry(name: &str) -> ObservedSubtree {
    let evidence = project_node(
        &read_of(PUSH_BUTTON, name, &[]),
        false,
        &WalkLimits::default(),
    );
    ObservedSubtree::new(evidence, Vec::new(), true, None)
}

impl Items for Fake {
    fn bind(&self, panel_titles: &[String]) -> Vec<Option<usize>> {
        self.log.borrow_mut().push(format!("bind {panel_titles:?}"));
        panel_titles
            .iter()
            .map(|title| self.titles.iter().position(|item| item == title))
            .collect()
    }

    fn decorate(
        &self,
        item: usize,
        evidence: &mut LocatorEvidence,
        first_child_index: usize,
    ) -> Vec<ObservedSubtree> {
        self.log
            .borrow_mut()
            .push(format!("decorate {item} at {first_child_index}"));
        evidence.name = LocatorField::Known(format!("item:{}", self.titles[item]));
        vec![entry("menu entry").with_source_child_index(first_child_index)]
    }

    fn mark_unbound(&self, evidence: &mut LocatorEvidence) {
        self.log.borrow_mut().push("unbound".to_owned());
        evidence.description = LocatorField::Known("unbound".to_owned());
    }

    fn unpaired(&self, bound: &[Option<usize>]) -> Vec<ObservedSubtree> {
        self.titles
            .iter()
            .enumerate()
            .filter(|(index, _)| !bound.contains(&Some(*index)))
            .map(|(_, title)| entry(&format!("unpaired:{title}")))
            .collect()
    }
}

fn node(
    read: ObservedRead,
    source_index: usize,
    children: Vec<WalkedNode<ObservedRead>>,
) -> WalkedNode<ObservedRead> {
    WalkedNode {
        payload: read,
        children,
        truncated: false,
        child_count: None,
        source_index,
        in_web_document: false,
    }
}

fn panel() -> Walk<ObservedRead> {
    let menus = ["Dropbox", "System", "Ghost"]
        .iter()
        .enumerate()
        .map(|(index, name)| node(read_of(MENU, name, &[]), index, Vec::new()))
        .collect();
    Walk {
        root: node(read_of(PANEL, "", &[]), 0, menus),
        complete: true,
        node_count: 4,
        max_depth: 1,
        counts: WalkCounts::default(),
    }
}

fn project(decorator: &mut dyn Decorate) -> AccessibilityNode {
    let (subtree, stats) = project_with(panel(), &WalkLimits::default(), decorator);
    let window = WindowInfo {
        id: "w-1.1-p".to_owned(),
        title: "t".to_owned(),
        app: "a".to_owned(),
        pid: 1.into(),
        process_instance: None,
        bounds: None,
        state: WindowState::default(),
    };
    let root = agent_desktop_core::ObservationRoot::Window(&window);
    ObservedTree::from_roots(
        vec![subtree],
        ObservationSource::from_root(&root, agent_desktop_core::SnapshotSurface::Window),
        stats,
        true,
    )
    .expect("builds")
    .into_accessibility_tree_partial()
    .expect("one root")
    .0
}

fn fake(titles: Vec<&'static str>, log: &Rc<RefCell<Vec<String>>>) -> Fake {
    Fake {
        titles,
        log: Rc::clone(log),
    }
}

#[test]
fn all_panel_titles_are_bound_at_once_and_the_system_menu_is_not_a_tray_item() {
    let log = Rc::default();
    let mut decorator = TrayDecorator::new(Some(fake(vec!["Dropbox"], &log)));
    project(&mut decorator);
    assert_eq!(log.borrow()[0], r#"bind ["Dropbox", "Ghost"]"#);
}

#[test]
fn a_bound_node_takes_the_item_and_its_menu_entries_follow_its_children() {
    let log = Rc::default();
    let mut decorator = TrayDecorator::new(Some(fake(vec!["Dropbox"], &log)));
    let tree = project(&mut decorator);
    assert_eq!(
        tree.children[0].identity.name.as_deref(),
        Some("item:Dropbox")
    );
    assert_eq!(tree.children[0].children.len(), 1);
}

#[test]
fn a_node_no_item_matched_is_marked_unbound_and_the_system_menu_is_left_alone() {
    let log = Rc::default();
    let mut decorator = TrayDecorator::new(Some(fake(vec!["Dropbox"], &log)));
    let tree = project(&mut decorator);
    assert_eq!(tree.children[1].identity.name.as_deref(), Some("System"));
    assert_eq!(
        log.borrow()
            .iter()
            .filter(|line| *line == "unbound")
            .count(),
        1
    );
}

#[test]
fn items_no_panel_node_showed_are_appended_after_the_panels_own_children() {
    let log = Rc::default();
    let mut decorator = TrayDecorator::new(Some(fake(vec!["Dropbox", "Hidden"], &log)));
    let tree = project(&mut decorator);
    let last = tree.children.last().expect("children");
    assert_eq!(last.identity.name.as_deref(), Some("unpaired:Hidden"));
    assert_eq!(tree.children.len(), 4);
}

#[test]
fn a_tray_that_could_not_be_read_leaves_the_panel_untouched() {
    let mut decorator: TrayDecorator<Fake> = TrayDecorator::new(None);
    let tree = project(&mut decorator);
    let names: Vec<_> = tree
        .children
        .iter()
        .map(|c| c.identity.name.clone().unwrap_or_default())
        .collect();
    assert_eq!(names, ["Dropbox", "System", "Ghost"]);
}

#[test]
fn the_system_button_is_given_the_click_capability_it_does_not_advertise() {
    let mut decorator: TrayDecorator<Fake> = TrayDecorator::new(None);
    let read = read_of(MENU, "System", &[]);
    let mut evidence = project_node(&read, false, &WalkLimits::default());
    assert_eq!(
        evidence.ref_evidence.available_actions,
        LocatorField::Known(Vec::new())
    );
    decorator.decorate(&read, &mut evidence, 0);
    assert_eq!(
        evidence.ref_evidence.available_actions,
        LocatorField::Known(vec![agent_desktop_core::capability::CLICK.to_owned()])
    );
}

#[test]
fn another_menu_node_keeps_the_capabilities_it_read() {
    let log = Rc::default();
    let mut decorator = TrayDecorator::new(Some(fake(vec!["Dropbox"], &log)));
    let read = read_of(MENU, "Dropbox", &[]);
    let mut evidence = project_node(&read, false, &WalkLimits::default());
    decorator.prepare(&node(read.clone(), 0, Vec::new()));
    decorator.decorate(&read, &mut evidence, 0);
    assert_eq!(
        evidence.ref_evidence.available_actions,
        LocatorField::Known(Vec::new())
    );
}
