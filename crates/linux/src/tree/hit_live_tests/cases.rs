use super::{
    AtspiElement, Strictness, candidates, exercise, gtk3_check, hit, verdict, window_extents,
};
use crate::tree::WalkLimits;
use crate::tree::live_apps::{AppSpec, app, qt, terminal, vscode};
use crate::tree::live_launch::nonce;
use crate::tree::live_rig::{deadline, rig};
use crate::tree::walker::walk_tree;

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn hit_live_gtk3_terminal_cover_in_both_directions() {
    let session = rig();
    terminal(&session.adapter, &session.bus, gtk3_check).expect("gtk3 terminal");
}

#[test]
#[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
fn hit_live_vscode_is_never_a_wrong_verdict() {
    let session = rig();
    vscode(&session.adapter, &session.bus, |_, _, _, frame| {
        exercise("vscode", Strictness::NeverWrong, frame)
    })
    .expect("vscode");
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
        println!("hit_live files: {} zero-axis nodes", zero.len());
        assert!(!zero.is_empty(), "the Files window has a zero-axis node");
        for element in zero.iter().take(5) {
            let outcome = hit(&inner, element, (300, 300));
            println!(
                "hit_live files: zero-axis {} -> {}",
                element.path(),
                verdict(&outcome)
            );
            assert_eq!(verdict(&outcome), "unknown");
        }
        Ok(())
    })
    .expect("files");
}
