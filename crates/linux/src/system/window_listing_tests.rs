use super::*;
use crate::system::process_facts::ProcessFacts;
use crate::system::registry_app::assemble;
use crate::system::toolkit_id::ToolkitId;
use crate::tree::test_support::{attributes, read_of, words};
use crate::tree::{AtspiElement, Extents, Field};

fn app(toolkit: Option<(&str, &str)>) -> RegistryApp {
    assemble(
        AtspiElement::new(":1.9", "/org/a11y/atspi/accessible/root", 321),
        ProcessFacts {
            executable: None,
            comm: "x".to_owned(),
            exe_basename: Some("x".to_owned()),
            instance: "boot:42".to_owned(),
            web_family: None,
        },
        Some(Some("App".to_owned())),
        ToolkitId {
            name: toolkit.map(|(name, _)| name.to_owned()),
            version: toolkit.map(|(_, version)| version.to_owned()),
        },
    )
}

fn frame(role: u32, name: &str, path: &str, state_bits: &[u32]) -> RawNode {
    let mut node = read_of(role, name, &[]).node;
    node.element = AtspiElement::new(":1.9", path, 321);
    node.traits.interfaces = Field::Present(vec![crate::tree::COMPONENT_INTERFACE.to_owned()]);
    node.traits.states = Field::Present(words(state_bits));
    node
}

#[test]
fn a_shown_active_frame_becomes_a_window_with_the_process_instance() {
    let app = app(Some(("gtk", "3.24")));
    let nodes = vec![frame(23, "Doc", "/org/a11y/atspi/accessible/1", &[25, 1])];
    let windows = collect(&[(0, &app)], &[nodes]);
    let [window] = windows.as_slice() else {
        panic!("one window")
    };
    assert_eq!(window.info.title, "Doc");
    assert_eq!(window.info.app, "x");
    assert_eq!(window.info.process_instance.as_deref(), Some("boot:42"));
    assert_eq!(window.info.state.visible, Some(true));
    assert!(window.active);
    assert!(window.frame.global_position);
    assert_eq!(window.info.bounds.map(|rect| rect.width), Some(10.0));
}

#[test]
fn non_window_roles_and_chrome_bubbles_are_dropped() {
    let app = app(Some(("Chromium", "1")));
    let mut bubble = frame(23, "", "/org/a11y/atspi/accessible/2", &[25]);
    bubble.attributes = attributes(&[("class", "RootView")]);
    let mut browser = frame(23, "Page", "/org/a11y/atspi/accessible/1", &[25]);
    browser.attributes = attributes(&[("class", "BrowserRootView")]);
    let nodes = vec![
        browser,
        bubble,
        frame(75, "Not a window", "/org/a11y/atspi/accessible/3", &[25]),
    ];
    let windows = collect(&[(0, &app)], &[nodes]);
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].info.title, "Page");
}

#[test]
fn an_open_chrome_menu_frame_is_not_a_window() {
    let app = app(Some(("Chromium", "1")));
    let mut menu = frame(23, "", "/org/a11y/atspi/accessible/4", &[25, 1]);
    menu.attributes = attributes(&[("class", "MenuHostRootView")]);
    let mut browser = frame(23, "Page", "/org/a11y/atspi/accessible/1", &[25]);
    browser.attributes = attributes(&[("class", "BrowserRootView")]);
    let windows = collect(&[(0, &app)], &[vec![browser, menu]]);
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].info.title, "Page");
    assert!(!windows[0].active);
}

#[test]
fn a_failed_state_read_leaves_visibility_unknown_and_never_active() {
    let app = app(None);
    let mut node = frame(23, "Doc", "/org/a11y/atspi/accessible/1", &[]);
    node.traits.states = Field::Failed("x".to_owned());
    node.extents = Field::Present(Extents {
        x: 0,
        y: 0,
        width: 5,
        height: 5,
    });
    let windows = collect(&[(0, &app)], &[vec![node]]);
    assert_eq!(windows[0].info.state.visible, None);
    assert!(!windows[0].active);
}

#[test]
fn a_frame_address_the_bus_could_not_have_issued_is_not_listed() {
    let app = app(None);
    let mut node = frame(23, "Doc", "/org/a11y/atspi/accessible/1", &[25]);
    node.element = AtspiElement::new("not-a-bus-name", "/p", 321);
    assert!(collect(&[(0, &app)], &[vec![node]]).is_empty());
}

#[test]
fn the_measured_about_and_shortcuts_shapes_are_listed_sheets() {
    let app = app(Some(("gtk", "4.14")));
    let nodes = vec![
        frame(23, "About", "/org/a11y/atspi/accessible/1", &[25, 16]),
        frame(16, "About Terminal", "/org/a11y/atspi/accessible/2", &[25]),
        frame(39, "", "/org/a11y/atspi/accessible/3", &[25, 16]),
        frame(39, "", "/org/a11y/atspi/accessible/4", &[25]),
        frame(23, "Doc", "/org/a11y/atspi/accessible/5", &[25]),
    ];
    let windows = collect(&[(0, &app)], &[nodes]);
    let sheets: Vec<bool> = windows.iter().map(|window| window.sheet).collect();
    assert_eq!(sheets, [true, true, true, false]);
}

#[test]
fn a_frame_that_neither_shows_nor_is_iconified_is_unshown_and_the_others_are_not() {
    let app = app(Some(("qt", "6.7")));
    let unshown = |bits: &[u32]| {
        let nodes = vec![frame(16, "d", "/org/a11y/atspi/accessible/1", bits)];
        collect(&[(0, &app)], &[nodes])[0].unshown
    };
    assert!(unshown(&[8, 24]));
    assert!(!unshown(&[8, 24, 25, 30]));
    assert!(!unshown(&[8, 15]));
}
