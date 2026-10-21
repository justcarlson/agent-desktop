mod cases;
mod x_tools;

use std::time::Duration;

use agent_desktop_core::{AdapterError, ErrorCode, HitTestResult, ObservationOps, Point};

use super::frame_geometry::pointer_bounds;
use super::live_rig::{Rig, Suspended, deadline, rig, wait_until};
use super::toolkit_trust::read_toolkit;
use super::walker::walk_tree;
use super::window_extents::read_window_extents;
use super::{AtspiElement, Extents, WalkedNode};
use super::{ObservedRead, Walk, WalkLimits};
use crate::LinuxAdapter;
use crate::system::SharedBus;
use crate::system::{REGISTRY_ROOT_PATH, within};
use x_tools::{Geometry, XWindow};

pub(super) fn verdict(result: &Result<HitTestResult, AdapterError>) -> &'static str {
    match result {
        Ok(HitTestResult::ReachesTarget) => "reaches",
        Ok(HitTestResult::InterceptedBy { .. }) => "intercepted",
        Ok(HitTestResult::Unknown) => "unknown",
        Err(_) => "error",
    }
}

pub(super) fn hit(
    rig: &Rig,
    target: &AtspiElement,
    point: (i32, i32),
) -> Result<HitTestResult, AdapterError> {
    let handle = target.clone().into_native_handle();
    let at = Point {
        x: f64::from(point.0),
        y: f64::from(point.1),
    };
    rig.adapter.hit_test(&handle, at, deadline(5_000))
}

fn polled(rig: &Rig, target: &AtspiElement, point: (i32, i32), wanted: &str) -> &'static str {
    let mut last = "unread";
    wait_until(Duration::from_secs(3), || {
        last = verdict(&hit(rig, target, point));
        last == wanted
    });
    last
}

/// Every node under the window with the window as its frame, the ancestry a
/// resolved ref carries.
pub(super) fn candidates(
    frame: &AtspiElement,
    node: &WalkedNode<super::ObservedRead>,
    out: &mut Vec<AtspiElement>,
) {
    for child in &node.children {
        let mut element = child.payload.node.element.clone();
        element.set_ancestry(super::ancestry::of_root(frame));
        out.push(element);
        candidates(frame, child, out);
    }
}

fn frame_title(rig: &Rig, frame: &AtspiElement) -> String {
    let budget = deadline(3_000);
    let connection = rig.bus.get(budget).expect("the bus is reachable");
    within(budget, async {
        Ok(super::node_read::read_node(connection.inner(), frame)
            .await
            .ok())
    })
    .ok()
    .flatten()
    .and_then(|node| node.properties.name.present().cloned())
    .unwrap_or_default()
}

pub(super) fn window_extents(rig: &Rig, element: &AtspiElement) -> Option<Extents> {
    read_window_extents(&rig.bus, element, deadline(3_000))
}

fn inside(frame: Extents, rect: Extents) -> bool {
    rect.width >= 30
        && rect.height >= 12
        && rect.x >= 4
        && rect.y >= 40
        && rect.x + rect.width <= frame.width - 4
        && rect.y + rect.height <= frame.height - 4
}

/// The window point a pointer would be aimed at, in root coordinates, from the
/// node's window extents and the X client origin `xwininfo` reports.
fn oracle_point(rect: Extents, origin: Geometry) -> (i32, i32) {
    (
        origin.x + rect.x + rect.width / 2,
        origin.y + rect.y + rect.height / 2,
    )
}

struct Scene {
    window: XWindow,
    target: AtspiElement,
    point: (i32, i32),
    baseline: &'static str,
}

fn scene(rig: &Rig, frame: &AtspiElement, walk: &Walk<ObservedRead>) -> Result<Scene, String> {
    let title = frame_title(rig, frame);
    let window = x_tools::window_of(frame.pid(), &title)
        .ok_or_else(|| "no X window for the application".to_owned())?;
    x_tools::raise(window.id);
    std::thread::sleep(Duration::from_millis(400));
    let origin = x_tools::geometry(window.id).ok_or("no geometry")?;
    let frame_rect = window_extents(rig, frame).ok_or("no frame extents")?;
    let mut all = Vec::new();
    candidates(frame, &walk.root, &mut all);
    let mut first = None;
    for element in all.into_iter().take(400) {
        let Some(rect) = window_extents(rig, &element) else {
            continue;
        };
        if !inside(frame_rect, rect) {
            continue;
        }
        let point = oracle_point(rect, origin);
        let outcome = verdict(&hit(rig, &element, point));
        if first.is_none() {
            first = Some((element.clone(), point, outcome));
        }
        if outcome == "reaches" {
            return Ok(Scene {
                window,
                target: element,
                point,
                baseline: outcome,
            });
        }
    }
    let (target, point, baseline) = first.ok_or("no node inside the window")?;
    Ok(Scene {
        window,
        target,
        point,
        baseline,
    })
}

fn cover(rig: &mut Rig, point: (i32, i32)) -> u32 {
    let geometry = format!(
        "500x400+{}+{}",
        (point.0 - 250).max(0),
        (point.1 - 200).max(0)
    );
    x_tools::appeared(|| {
        rig.launched
            .spawn("xclock", &["-geometry", &geometry, "-update", "1"]);
    })
    .expect("the cover window appears")
}

/// Both directions of the cross-window leg: an ordinary cover intercepts until
/// the target is raised; an `ABOVE` cover keeps intercepting through a raise
/// and stops only when it is lowered. Returns one line per measurement.
fn both_directions(rig: &mut Rig, scene: &Scene) -> Vec<(String, &'static str, &'static str)> {
    let mut rows = Vec::new();
    let mut record = |step: &str, wanted: &'static str, got: &'static str| {
        rows.push((step.to_owned(), wanted, got));
    };
    let (target, point) = (&scene.target, scene.point);
    record(
        "uncovered",
        "reaches",
        polled(rig, target, point, "reaches"),
    );
    let covering = cover(rig, point);
    record(
        "ordinary cover",
        "intercepted",
        polled(rig, target, point, "intercepted"),
    );
    x_tools::raise(scene.window.id);
    record(
        "target raised",
        "reaches",
        polled(rig, target, point, "reaches"),
    );
    x_tools::raise(covering);
    x_tools::set_above(covering, true);
    record(
        "ABOVE cover",
        "intercepted",
        polled(rig, target, point, "intercepted"),
    );
    x_tools::raise(scene.window.id);
    record(
        "target raised under ABOVE",
        "intercepted",
        polled(rig, target, point, "intercepted"),
    );
    x_tools::set_above(covering, false);
    x_tools::lower(covering);
    x_tools::raise(scene.window.id);
    record(
        "cover lowered",
        "reaches",
        polled(rig, target, point, "reaches"),
    );
    rows
}

fn report(label: &str, scene: &Scene, rows: &[(String, &'static str, &'static str)]) {
    println!(
        "hit_live {label}: baseline {} at {:?} target {}{}",
        scene.baseline,
        scene.point,
        scene.target.bus_name(),
        scene.target.path()
    );
    for (step, wanted, got) in rows {
        println!("hit_live {label}: {step}: wanted {wanted} got {got}");
    }
}

fn assert_strict(label: &str, rows: &[(String, &'static str, &'static str)]) {
    for (step, wanted, got) in rows {
        assert_eq!(got, wanted, "{label}: {step}");
    }
}

fn assert_never_wrong(label: &str, rows: &[(String, &'static str, &'static str)]) {
    for (step, wanted, got) in rows {
        assert!(
            got == wanted || *got == "unknown",
            "{label}: {step}: wanted {wanted} or unknown, got {got}"
        );
    }
}

/// The frame's pointer rectangle against what the X server says about the
/// client window, and whether the toolkit's own screen rectangle agrees.
fn frame_against_oracle(
    rig: &Rig,
    frame: &AtspiElement,
    walk: &Walk<ObservedRead>,
    window: &XWindow,
) -> String {
    let origin = x_tools::geometry(window.id).expect("the client window has geometry");
    let node = walk.root.payload.node.clone();
    let screen = *node
        .extents
        .present()
        .expect("the frame has screen extents");
    let own = window_extents(rig, frame).expect("the frame has window extents");
    let budget = deadline(3_000);
    let connection = rig.bus.get(budget).expect("the bus is reachable");
    let root = AtspiElement::new(frame.bus_name(), REGISTRY_ROOT_PATH, frame.pid());
    let id = within(budget, async {
        Ok::<_, AdapterError>(read_toolkit(connection.inner(), &root).await)
    })
    .expect("the toolkit reads");
    let bounds = pointer_bounds(&id, screen, own, Some((origin.x, origin.y)));
    format!(
        "toolkit {:?} {:?}: screen {screen:?} window {own:?} xwininfo {origin:?} pointer {bounds:?}",
        id.name, id.version
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Strictness {
    Exact,
    NeverWrong,
}

pub(super) fn exercise(
    label: &str,
    strictness: Strictness,
    frame: &AtspiElement,
) -> Result<(), String> {
    let mut rig = rig();
    let walk = walk_tree(&rig.bus, frame, &WalkLimits::default(), deadline(10_000))
        .map_err(|error| error.message)?;
    let scene = scene(&rig, frame, &walk)?;
    println!(
        "hit_live {label}: {}",
        frame_against_oracle(&rig, frame, &walk, &scene.window)
    );
    let rows = both_directions(&mut rig, &scene);
    report(label, &scene, &rows);
    match strictness {
        Strictness::Exact => assert_strict(label, &rows),
        Strictness::NeverWrong => assert_never_wrong(label, &rows),
    }
    minimized_and_stopped(label, &rig, &scene);
    Ok(())
}

/// A minimized window is never reachable, and an application held stopped
/// answers with the timeout inside the budget and never with a verdict.
fn minimized_and_stopped(label: &str, rig: &Rig, scene: &Scene) {
    x_tools::minimize(scene.window.id);
    let minimized = polled(rig, &scene.target, scene.point, "unknown");
    println!("hit_live {label}: minimized: {minimized}");
    assert_eq!(minimized, "unknown", "{label}: minimized");
    x_tools::raise(scene.window.id);
    std::thread::sleep(Duration::from_millis(500));
    let pid = scene.target.pid();
    let stopped = Suspended::new(scene.window.pid);
    let started = std::time::Instant::now();
    let outcome = rig.adapter.hit_test(
        &scene.target.clone().into_native_handle(),
        Point {
            x: f64::from(scene.point.0),
            y: f64::from(scene.point.1),
        },
        deadline(2_000),
    );
    let elapsed = started.elapsed();
    drop(stopped);
    println!("hit_live {label}: stopped pid {pid}: {outcome:?} in {elapsed:?}");
    let error = outcome.expect_err("a stopped application yields no verdict");
    assert!(
        matches!(error.code, ErrorCode::AppUnresponsive | ErrorCode::Timeout),
        "{label}: stopped: {:?}",
        error.code
    );
    assert!(
        elapsed < Duration::from_millis(2_600),
        "{label}: {elapsed:?}"
    );
}

pub(super) fn gtk3_check(
    _: &str,
    _: &LinuxAdapter,
    _: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    exercise("gtk3 terminal", Strictness::Exact, frame)
}

pub(super) fn gtk4_check(
    label: &str,
    _: &LinuxAdapter,
    _: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    let rig = rig();
    let title = frame_title(&rig, frame);
    let window = x_tools::window_of(frame.pid(), &title).ok_or("no X window")?;
    for (x, y) in [(60, 60), (380, 190)] {
        x_tools::move_to(window.id, x, y);
        std::thread::sleep(Duration::from_millis(500));
        println!(
            "hit_live {label} moved to {x},{y}: {:?}",
            x_tools::geometry(window.id)
        );
        exercise(
            &format!("{label} at {x},{y}"),
            Strictness::NeverWrong,
            frame,
        )?;
    }
    Ok(())
}
