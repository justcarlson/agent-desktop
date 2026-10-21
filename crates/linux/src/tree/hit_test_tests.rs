use super::*;
use crate::system::x11::FrameExtents;
use crate::system::x11::desktop_placement::DesktopPlacement;
use crate::system::x11::stacked_window::StackedWindow;
use crate::system::x11::window_flags::WindowFlags;
use crate::system::x11::window_kind::WindowKind;
use crate::tree::hit_cover::tests::client;
use crate::tree::hit_descent::tests::{FakeNodes, el};
use crate::tree::hit_descent::{Described, Hit};

const CLIENT: u32 = 10;

pub(crate) fn rect(x: i32, y: i32, width: i32, height: i32) -> Extents {
    Extents {
        x,
        y,
        width,
        height,
    }
}

fn toolkit(name: &str, version: &str) -> ToolkitId {
    ToolkitId {
        name: Some(name.to_owned()),
        version: Some(version.to_owned()),
    }
}

fn frame_facts() -> FrameFacts {
    FrameFacts {
        pid: 1,
        title: "Terminal".to_owned(),
        extents: None,
        global_position: true,
    }
}

fn gtk3_facts() -> Facts {
    Facts {
        screen: rect(100, 137, 420, 34),
        window: rect(0, 0, 420, 34),
        frame: frame_facts(),
        minimized: false,
        toolkit: toolkit("gtk", "3.24.41"),
    }
}

fn gtk4_facts() -> Facts {
    Facts {
        screen: rect(0, 0, 558, 54),
        window: rect(329, 285, 558, 54),
        frame: frame_facts(),
        minimized: false,
        toolkit: toolkit("gtk", "4.14.5"),
    }
}

fn stacked(id: u32, client: Extents) -> StackedWindow {
    StackedWindow {
        shadow: (0, 0, 0, 0),
        id,
        flags: WindowFlags {
            hidden: false,
            above: false,
            mapped: true,
        },
        desktop: DesktopPlacement::On(0),
        kind: WindowKind::Normal,
        client,
        extents: FrameExtents {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        },
    }
}

fn windowing(origin: (i32, i32), above: Vec<StackedWindow>) -> Windowing {
    let mut stack = vec![stacked(CLIENT, rect(origin.0, origin.1, 800, 600))];
    stack.extend(above);
    Windowing {
        coordinate_origin: None,
        client: CLIENT,
        snapshot: StackingSnapshot {
            current_desktop: Some(0),
            stack,
            override_redirect_above: Vec::new(),
            override_redirect_receiving: Vec::new(),
            override_redirect_names: Vec::new(),
        },
        clients: Vec::new(),
    }
}

struct World {
    nodes: FakeNodes,
    facts: Option<Facts>,
    windowing: Option<Windowing>,
    windowing_reads: usize,
}

impl World {
    fn new(facts: Facts, windowing: Option<Windowing>) -> Self {
        let mut nodes = FakeNodes::default();
        nodes
            .parent("/pane", "/frame")
            .parent("/target", "/pane")
            .parent("/other", "/frame");
        Self {
            nodes,
            facts: Some(facts),
            windowing,
            windowing_reads: 0,
        }
    }
}

impl NodeSource for World {
    fn at_point(&mut self, from: &AtspiElement, point: (i32, i32)) -> Result<Hit, AdapterError> {
        self.nodes.at_point(from, point)
    }

    fn verified_parent(
        &mut self,
        node: &AtspiElement,
    ) -> Result<Option<AtspiElement>, AdapterError> {
        self.nodes.verified_parent(node)
    }

    fn describe(&mut self, node: &AtspiElement) -> Result<Described, AdapterError> {
        self.nodes.describe(node)
    }
}

impl HitWorld for World {
    fn facts(
        &mut self,
        _target: &AtspiElement,
        _frame: &AtspiElement,
    ) -> Result<Option<Facts>, AdapterError> {
        Ok(self.facts.clone())
    }

    fn windowing(&mut self, _frame: &FrameFacts) -> Option<Windowing> {
        self.windowing_reads += 1;
        self.windowing.clone()
    }
}

fn test(world: &mut World, point: (i32, i32)) -> HitTestResult {
    let deadline = Deadline::after(5_000).expect("a deadline builds");
    hit_test_via(world, &el("/target"), &el("/frame"), point, deadline).expect("no timeout")
}

fn reaching(world: &mut World) {
    world
        .nodes
        .lead("/frame", "/pane")
        .lead("/pane", "/target")
        .lead("/target", "/target");
}

#[test]
fn a_gtk3_target_the_descent_reaches_with_a_clear_stack_reaches_target() {
    let mut world = World::new(gtk3_facts(), Some(windowing((100, 137), vec![])));
    reaching(&mut world);
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::ReachesTarget);
    assert_eq!(world.nodes.points.first(), Some(&(210, 17)));
}

#[test]
fn a_window_above_the_target_over_the_point_intercepts_without_a_descent() {
    let cover = stacked(20, rect(200, 100, 600, 400));
    let mut world = World::new(gtk3_facts(), Some(windowing((100, 137), vec![cover])));
    reaching(&mut world);
    let HitTestResult::InterceptedBy { role, name, bounds } = test(&mut world, (310, 154)) else {
        panic!("a cover must intercept");
    };
    assert_eq!(role.as_deref(), Some("window"));
    assert_eq!(name, None);
    assert_eq!(bounds.expect("the cover's frame").width, 600.0);
    assert!(world.nodes.calls.is_empty());
}

#[test]
fn a_cover_that_maps_to_one_client_is_named_by_its_title() {
    let cover = stacked(20, rect(200, 100, 600, 400));
    let mut covered = windowing((100, 137), vec![cover]);
    covered.clients = vec![client(CLIENT, 77, "Target"), client(20, 77, "Cover")];
    let mut world = World::new(gtk3_facts(), Some(covered));
    let HitTestResult::InterceptedBy { name, .. } = test(&mut world, (310, 154)) else {
        panic!("a cover must intercept");
    };
    assert_eq!(name.as_deref(), Some("Cover"));
}

#[test]
fn the_cross_window_leg_is_what_separates_reaches_from_intercepted() {
    for (name, version) in [("gtk", "3.24.41"), ("Qt", "6.10"), ("Chromium", "1.0")] {
        let mut facts = gtk3_facts();
        facts.toolkit = toolkit(name, version);
        let covered = stacked(20, rect(200, 100, 600, 400));
        let mut world = World::new(facts.clone(), Some(windowing((100, 137), vec![covered])));
        reaching(&mut world);
        let result = test(&mut world, (310, 154));
        assert!(matches!(result, HitTestResult::InterceptedBy { .. }));
        let mut raised = World::new(facts, Some(windowing((100, 137), vec![])));
        reaching(&mut raised);
        assert_eq!(test(&mut raised, (310, 154)), HitTestResult::ReachesTarget);
        assert_eq!(raised.nodes.points.first(), Some(&(210, 17)));
    }
}

#[test]
fn a_cover_that_misses_the_point_leaves_the_target_reachable() {
    let aside = stacked(20, rect(700, 700, 50, 50));
    let mut world = World::new(gtk3_facts(), Some(windowing((100, 137), vec![aside])));
    reaching(&mut world);
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::ReachesTarget);
}

#[test]
fn an_unrelated_lander_on_a_trusted_toolkit_intercepts_with_its_role_name_and_bounds() {
    let mut world = World::new(gtk3_facts(), Some(windowing((100, 137), vec![])));
    world.nodes.lead("/frame", "/other");
    world.nodes.described.insert(
        "/other".to_owned(),
        Described {
            role: Some("push button".to_owned()),
            name: Some("Close".to_owned()),
            window_extents: Some(rect(200, 10, 40, 20)),
        },
    );
    let HitTestResult::InterceptedBy { role, name, bounds } = test(&mut world, (310, 154)) else {
        panic!("a trusted lander intercepts");
    };
    assert_eq!(role.as_deref(), Some("push button"));
    assert_eq!(name.as_deref(), Some("Close"));
    let bounds = bounds.expect("bounds");
    assert_eq!((bounds.x, bounds.y), (300.0, 147.0));
}

#[test]
fn a_lander_on_an_untrusted_toolkit_is_unknown() {
    let mut facts = gtk3_facts();
    facts.toolkit = toolkit("Chromium", "1.0");
    let mut world = World::new(facts, Some(windowing((100, 137), vec![])));
    world.nodes.lead("/frame", "/other");
    world.nodes.described.insert(
        "/other".to_owned(),
        Described {
            window_extents: Some(rect(200, 10, 40, 20)),
            ..Described::default()
        },
    );
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
}

#[test]
fn an_ancestor_landing_is_unknown() {
    let mut world = World::new(gtk3_facts(), Some(windowing((100, 137), vec![])));
    world.nodes.lead("/frame", "/pane");
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
}

#[test]
fn an_endless_descent_is_unknown() {
    let mut world = World::new(gtk3_facts(), Some(windowing((100, 137), vec![])));
    world.nodes.endless = true;
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
}

#[test]
fn an_override_redirect_window_over_the_point_is_unknown() {
    let mut stacking = windowing((100, 137), vec![]);
    stacking.snapshot.override_redirect_above = vec![rect(300, 150, 50, 50)];
    let mut world = World::new(gtk3_facts(), Some(stacking));
    reaching(&mut world);
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
}

#[test]
fn x11_that_cannot_be_read_is_unknown() {
    let mut world = World::new(gtk3_facts(), None);
    reaching(&mut world);
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
}

#[test]
fn a_global_toolkit_whose_origin_disagrees_with_x11_is_unknown() {
    let mut world = World::new(gtk3_facts(), Some(windowing((400, 300), vec![])));
    reaching(&mut world);
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
}

#[test]
fn gtk4_with_a_unique_mapping_reaches_the_target_at_the_calibrated_position() {
    let mut world = World::new(gtk4_facts(), Some(windowing((302, 260), vec![])));
    reaching(&mut world);
    assert_eq!(test(&mut world, (700, 560)), HitTestResult::ReachesTarget);
    assert_eq!(world.nodes.points.first(), Some(&(398, 300)));
}

#[test]
fn gtk4_without_a_mapping_is_unknown_for_every_input() {
    let mut world = World::new(gtk4_facts(), None);
    reaching(&mut world);
    assert_eq!(test(&mut world, (700, 560)), HitTestResult::Unknown);
    let mut covered = World::new(gtk4_facts(), None);
    assert_eq!(test(&mut covered, (700, 560)), HitTestResult::Unknown);
}

#[test]
fn a_point_inside_the_raw_gtk4_rectangle_but_outside_the_calibrated_one_is_unknown() {
    let mut world = World::new(gtk4_facts(), Some(windowing((302, 260), vec![])));
    reaching(&mut world);
    assert_eq!(test(&mut world, (10, 10)), HitTestResult::Unknown);
    assert!(world.nodes.calls.is_empty());
}

#[test]
fn a_calibrated_gtk4_target_under_a_cover_is_intercepted_by_it() {
    let cover = stacked(20, rect(500, 400, 800, 400));
    let mut world = World::new(gtk4_facts(), Some(windowing((302, 260), vec![cover])));
    let result = test(&mut world, (700, 560));
    assert!(matches!(result, HitTestResult::InterceptedBy { .. }));
}

#[test]
fn a_gtk4_lander_elsewhere_in_the_window_is_unknown() {
    let mut world = World::new(gtk4_facts(), Some(windowing((302, 260), vec![])));
    world.nodes.lead("/frame", "/other");
    world.nodes.described.insert(
        "/other".to_owned(),
        Described {
            window_extents: Some(rect(300, 250, 300, 100)),
            ..Described::default()
        },
    );
    assert_eq!(test(&mut world, (700, 560)), HitTestResult::Unknown);
}

#[test]
fn a_minimized_window_is_unknown_without_reading_x11_or_the_bus() {
    let mut facts = gtk3_facts();
    facts.minimized = true;
    let mut world = World::new(facts, Some(windowing((100, 137), vec![])));
    reaching(&mut world);
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
    assert_eq!(world.windowing_reads, 0);
    assert!(world.nodes.calls.is_empty());
}

#[test]
fn zero_size_and_sentinel_targets_are_unknown_before_any_read() {
    for screen in [
        rect(0, 0, 710, 0),
        rect(i32::MIN, i32::MIN, 100, 100),
        rect(-1, -1, -1, -1),
    ] {
        let mut facts = gtk3_facts();
        facts.screen = screen;
        let mut world = World::new(facts, Some(windowing((100, 137), vec![])));
        reaching(&mut world);
        assert_eq!(test(&mut world, (0, 0)), HitTestResult::Unknown);
        assert_eq!(world.windowing_reads, 0);
        assert!(world.nodes.calls.is_empty());
    }
}

#[test]
fn a_point_40_pixels_right_of_the_button_is_unknown() {
    let mut world = World::new(gtk3_facts(), Some(windowing((100, 137), vec![])));
    reaching(&mut world);
    assert_eq!(test(&mut world, (560, 154)), HitTestResult::Unknown);
}

#[test]
fn unreadable_facts_are_unknown() {
    let mut world = World::new(gtk3_facts(), Some(windowing((100, 137), vec![])));
    world.facts = None;
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
}

#[path = "hit_test_shadow_tests.rs"]
mod shadow;

#[path = "hit_test_popup_tests.rs"]
mod popup;
