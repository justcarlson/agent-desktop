use agent_desktop_core::ref_alloc::is_ref_able;
use agent_desktop_core::{
    AccessibilityNode, Deadline, ObservationRequest, ObservationRoot, TreeOptions, WindowInfo,
    WindowState,
};

use super::live_launch::Launched;
use super::live_tree::pid_frame;
use super::observe::observe_tree;
use super::walker_live_tests::settle;
use crate::system::{SharedBus, window_id};

fn refs(node: &AccessibilityNode) -> usize {
    usize::from(is_ref_able(node)) + node.children.iter().map(refs).sum::<usize>()
}

fn nodes(node: &AccessibilityNode) -> usize {
    1 + node.children.iter().map(nodes).sum::<usize>()
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/remote-linux.sh"]
fn observe_live_calculator_returns_a_complete_reffed_tree() {
    let bus = SharedBus::default();
    let connection = bus
        .get(Deadline::standard().expect("a standard deadline builds"))
        .expect("the accessibility bus must be reachable");
    let mut launched = Launched::new();
    let pid = launched.spawn("gnome-calculator", &[]);
    let settled = settle(&bus, 30, || pid_frame(&connection, pid))
        .unwrap_or_else(|error| panic!("calculator did not settle: {error}"));
    let id = window_id::encode(settled.frame.bus_name(), settled.frame.path())
        .expect("a registry frame has an encodable address");
    let window = WindowInfo {
        id,
        title: String::new(),
        app: "calculator".to_owned(),
        pid: pid.into(),
        process_instance: None,
        bounds: None,
        state: WindowState::default(),
    };
    let deadline = Deadline::after(10_000).expect("the deadline builds");
    let request = ObservationRequest::snapshot(&TreeOptions::default(), deadline);
    let tree = observe_tree(&bus, ObservationRoot::Window(&window), &request)
        .expect("the window observes");
    assert!(tree.is_complete());
    let (root, complete, observed) = tree
        .into_accessibility_tree_partial()
        .expect("one root projects");
    println!(
        "observe_live calculator: nodes={} observed={observed} refs={} complete={complete} root_role={}",
        nodes(&root),
        refs(&root),
        root.role
    );
    assert!(complete);
    assert!(refs(&root) >= 20, "a calculator has many buttons");
}
