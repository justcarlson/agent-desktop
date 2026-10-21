use super::{AtspiElement, candidates, hit, window_extents};
use crate::LinuxAdapter;
use crate::system::SharedBus;
use crate::tree::WalkLimits;
use crate::tree::live_apps::{AppSpec, app, terminal};
use crate::tree::live_rig::{Suspended, deadline, rig};
use crate::tree::walker::walk_tree;
use agent_desktop_core::{ErrorCode, ObservationOps, Point};
use std::time::{Duration, Instant};

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn stopped_application_returns_bounded_timeout() {
    let session = rig();
    terminal(&session.adapter, &session.bus, stopped_check).expect("gtk3 terminal");
}

fn stopped_check(
    _: &str,
    adapter: &LinuxAdapter,
    _: &SharedBus,
    frame: &AtspiElement,
) -> Result<(), String> {
    let mut target = frame.clone();
    target.set_ancestry(crate::tree::ancestry::of_root(frame));
    let handle = target.into_native_handle();
    let started = Instant::now();
    let stopped = Suspended::new(frame.pid());
    let outcome = adapter.hit_test(&handle, Point { x: 300.0, y: 300.0 }, deadline(2_000));
    let elapsed = started.elapsed();
    drop(stopped);

    let error = outcome.expect_err("a stopped application yields no verdict");
    assert!(
        matches!(error.code, ErrorCode::AppUnresponsive | ErrorCode::Timeout),
        "stopped application returned {:?}",
        error.code
    );
    assert!(
        elapsed < Duration::from_millis(2_600),
        "stopped application exceeded its response budget: {elapsed:?}"
    );
    Ok(())
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn hit_live_files_zero_axis_group_is_unknown() {
    let session = rig();
    let spec = AppSpec {
        label: "files",
        program: "nautilus",
        arguments: &["--new-window"],
        min_nodes: 30,
    };
    app(&spec, &session.adapter, &session.bus, |_, _, _, frame| {
        let inner = rig();
        let walk = walk_tree(&inner.bus, frame, &WalkLimits::default(), deadline(10_000))
            .map_err(|error| error.message)?;
        let mut all = Vec::new();
        candidates(frame, &walk.root, &mut all);
        let zero: Vec<AtspiElement> = all
            .into_iter()
            .filter(|element| {
                window_extents(&inner, element)
                    .is_some_and(|rect| rect.width == 0 || rect.height == 0)
            })
            .collect();
        assert!(!zero.is_empty(), "the Files window has a zero-axis node");
        for element in zero.iter().take(5) {
            assert_eq!(
                hit(&inner, element, (300, 300))
                    .expect("zero-axis targets produce a hit-test result"),
                agent_desktop_core::HitTestResult::Unknown,
                "zero-axis node {} must remain unknown",
                element.path()
            );
        }
        Ok(())
    })
    .expect("files");
}
