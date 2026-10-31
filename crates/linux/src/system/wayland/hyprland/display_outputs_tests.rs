use agent_desktop_core::Deadline;

use super::*;
use crate::system::wayland::hyprland::fake_compositor::{FakeCompositor, FakeOutput, Script};
use crate::system::wayland::hyprland::wayland_fault::WaylandFault;

fn deadline() -> Deadline {
    Deadline::after(2_000).expect("deadline")
}

fn fake(name: &'static str, position: (i32, i32), transform: i32) -> FakeOutput {
    FakeOutput {
        name,
        position,
        logical: (1920, 1200),
        mode: (2880, 1800),
        transform,
        scale: 2,
    }
}

fn read(
    globals: Vec<(&'static str, u32)>,
    outputs: Vec<FakeOutput>,
) -> Result<Vec<WaylandOutput>, OutputProbe> {
    let compositor = FakeCompositor::start(Script::Answer { globals, outputs });
    read_outputs(&compositor.socket, deadline())
}

#[test]
fn every_advertised_output_is_read_with_its_name_layout_mode_and_scale() {
    let outputs = read(
        vec![
            ("wl_output", 4),
            ("zxdg_output_manager_v1", 3),
            ("wl_output", 4),
        ],
        vec![fake("eDP-1", (0, 0), 0), fake("DP-2", (1920, 0), 1)],
    )
    .expect("outputs read");
    assert_eq!(
        outputs,
        vec![
            WaylandOutput {
                name: Some("eDP-1".to_owned()),
                position: Some((0, 0)),
                logical: Some((1920, 1200)),
                mode: Some((2880, 1800)),
                turned: false,
                integer_scale: 2,
            },
            WaylandOutput {
                name: Some("DP-2".to_owned()),
                position: Some((1920, 0)),
                logical: Some((1920, 1200)),
                mode: Some((2880, 1800)),
                turned: true,
                integer_scale: 2,
            },
        ]
    );
}

#[test]
fn a_first_version_manager_takes_the_name_from_wl_output() {
    let outputs = read(
        vec![("zxdg_output_manager_v1", 1), ("wl_output", 4)],
        vec![fake("HDMI-A-1", (0, 0), 0)],
    )
    .expect("outputs read");
    assert_eq!(outputs[0].name.as_deref(), Some("HDMI-A-1"));
    assert_eq!(outputs[0].position, Some((0, 0)));
}

#[test]
fn a_third_version_output_takes_the_name_from_xdg_output() {
    let outputs = read(
        vec![("zxdg_output_manager_v1", 2), ("wl_output", 3)],
        vec![fake("DP-3", (0, 0), 0)],
    )
    .expect("outputs read");
    assert_eq!(outputs[0].name.as_deref(), Some("DP-3"));
}

#[test]
fn a_compositor_that_names_no_output_leaves_the_name_empty() {
    let outputs = read(
        vec![("zxdg_output_manager_v1", 1), ("wl_output", 3)],
        vec![fake("HDMI-A-1", (0, 0), 0)],
    )
    .expect("outputs read");
    assert_eq!(outputs[0].name, None);
    assert_eq!(outputs[0].logical, Some((1920, 1200)));
}

#[test]
fn the_probe_names_each_global_the_compositor_does_not_advertise() {
    let output = || vec![fake("DP-1", (0, 0), 0)];
    assert_eq!(
        read(vec![("wl_output", 4)], output()),
        Err(OutputProbe::Unadvertised(vec!["zxdg_output_manager_v1"]))
    );
    assert_eq!(
        read(vec![("zxdg_output_manager_v1", 3)], vec![]),
        Err(OutputProbe::Unadvertised(vec!["wl_output"]))
    );
    assert_eq!(
        read(vec![("wl_compositor", 6)], vec![]),
        Err(OutputProbe::Unadvertised(vec![
            "wl_output",
            "zxdg_output_manager_v1"
        ]))
    );
}

#[test]
fn a_silent_compositor_ends_the_read_as_a_wayland_fault() {
    let compositor = FakeCompositor::start(Script::Silent);
    let answer = read_outputs(&compositor.socket, Deadline::after(150).expect("deadline"));
    assert_eq!(answer, Err(OutputProbe::Wayland(WaylandFault::Silent)));
}
