use std::time::{Duration, Instant};

use agent_desktop_core::{
    CursorOverlayConfig, CursorOverlayControl, CursorOverlayInstruction, CursorOverlayStyle, Point,
};

use super::{holder_reply, refresh_holders};
use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::hyprland::cursor_renderer::fake_layer_compositor::FakeLayerCompositor;
use crate::system::hyprland::cursor_renderer::holder_file::now_ms;
use crate::system::hyprland::cursor_renderer::holder_lease::HolderLease;
use crate::system::hyprland::cursor_renderer::holder_watch::HolderWatch;
use crate::system::hyprland::cursor_renderer::keep_drawn_flag::KEEP_DRAWN_FLAG;
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;
use crate::system::hyprland::cursor_renderer::renderer_process::RendererProcess;
use crate::system::hyprland::cursor_renderer::renderer_reply::Flow;
use crate::system::hyprland::cursor_renderer::renderer_request::RendererRequest;
use crate::system::hyprland::cursor_renderer::renderer_target::RendererTarget;
use crate::system::hyprland::cursor_renderer::renderer_wire::exchange;

fn process(fixture: &RegistryFixture, fake: &FakeLayerCompositor) -> RendererProcess {
    std::fs::create_dir_all(fixture.root().join("hypr").join("fixture-signature"))
        .expect("compositor dir");
    let target = RendererTarget {
        binding: HyprlandBinding {
            signature: "fixture-signature".into(),
            ipc_socket: fixture.root().join(".socket.sock"),
            wayland_socket: fake.socket.clone(),
            pid: 1,
            start_time: 1,
        },
        runtime_dir: fixture.root().to_path_buf(),
    };
    let enable =
        CursorOverlayControl::enable("fixture-session".into(), CursorOverlayStyle::default());
    RendererProcess::start_over(fake.connect(), fixture.paths.clone(), &target, &enable)
        .expect("renderer starts")
}

#[test]
fn the_renderer_answers_a_live_holders_hide_and_keeps_serving() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let mut process = process(&fixture, &fake);
    let mut watch = HolderWatch::loaded(&fixture.paths.holders);
    let held = HolderLease::take(&fixture.paths.holders, now_ms() + 5_000).expect("lease");
    let hide = RendererRequest::Hide {
        holder: held.nonce().to_owned(),
        expiry_ms: held.expiry(),
    };
    let (reply, flow) = holder_reply(&mut process, &mut watch, &hide);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(flow, Flow::Continue);
    assert!(watch.is_suppressed());
    let show = RendererRequest::Show {
        holder: held.nonce().to_owned(),
    };
    drop(held);
    let (reply, _) = holder_reply(&mut process, &mut watch, &show);
    assert_eq!(reply["ok"], true, "{reply}");
    assert!(!watch.is_suppressed());
    refresh_holders(&mut process, &mut watch).expect("idle tick");
}

#[test]
fn the_renderer_refuses_a_hide_for_a_holder_it_cannot_read() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let mut process = process(&fixture, &fake);
    let mut watch = HolderWatch::default();
    let hide = RendererRequest::Hide {
        holder: "missing".into(),
        expiry_ms: now_ms() + 5_000,
    };
    let (reply, flow) = holder_reply(&mut process, &mut watch, &hide);
    assert_eq!(reply["error"]["kind"], "cursor_holder_not_live");
    assert_eq!(flow, Flow::Continue);
}

#[test]
fn a_hide_under_the_keep_drawn_flag_says_the_cursor_stayed_drawn() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let mut process = process(&fixture, &fake);
    let mut watch = HolderWatch::default();
    let held = HolderLease::take(&fixture.paths.holders, now_ms() + 5_000).expect("lease");
    let hide = RendererRequest::Hide {
        holder: held.nonce().to_owned(),
        expiry_ms: held.expiry(),
    };
    let (plain, _) = holder_reply(&mut process, &mut watch, &hide);
    assert!(plain.get("kept_drawn_for_test").is_none(), "{plain}");
    std::fs::write(fixture.paths.dir.join(KEEP_DRAWN_FLAG), b"").expect("flag");
    let (kept, flow) = holder_reply(&mut process, &mut watch, &hide);
    assert_eq!(kept["ok"], true, "{kept}");
    assert_eq!(kept["kept_drawn_for_test"], true, "{kept}");
    assert_eq!(flow, Flow::Continue);
}

fn margins(fake: &FakeLayerCompositor) -> usize {
    fake.logged()
        .iter()
        .filter(|line| line.starts_with("margin "))
        .count()
}

fn served(fixture: &RegistryFixture, request: &RendererRequest) -> serde_json::Value {
    let soon = Instant::now() + Duration::from_secs(3);
    exchange(&fixture.paths.socket, request, soon)
        .expect("exchange")
        .expect("renderer listens")
}

#[test]
fn a_travel_under_a_holders_hide_lands_unseen_and_show_draws_it_there() {
    let fixture = RegistryFixture::new();
    let fake = FakeLayerCompositor::start(true);
    let renderer = std::thread::spawn({
        let process = process(&fixture, &fake).with_wanted(|_| true);
        move || process.serve()
    });
    let config = CursorOverlayConfig::enabled(None, 6).expect("config");
    let to = Point {
        x: 1200.0,
        y: 900.0,
    };
    let instruction = CursorOverlayInstruction::new(to, &config, true).expect("instruction");
    let travel = RendererRequest::control(CursorOverlayControl::present(
        "fixture-session".into(),
        instruction,
    ));
    let socket = fixture.paths.socket.clone();
    let shown = margins(&fake);
    let travelling = std::thread::spawn(move || {
        exchange(&socket, &travel, Instant::now() + Duration::from_secs(3))
    });
    let started = Instant::now();
    while margins(&fake) == shown {
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "no travel frame"
        );
        std::thread::yield_now();
    }
    let held = HolderLease::take(&fixture.paths.holders, now_ms() + 5_000).expect("lease");
    let before_hide = fake.logged().len();
    let hide = RendererRequest::Hide {
        holder: held.nonce().to_owned(),
        expiry_ms: held.expiry(),
    };
    assert_eq!(served(&fixture, &hide)["ok"], true);
    let hid = fake.logged()[before_hide..].to_vec();
    let cleared = hid.iter().position(|line| line == "commit").expect("clear");
    assert!(hid[cleared..].iter().any(|line| line == "sync"), "{hid:?}");
    let hidden = margins(&fake);
    let landed = travelling.join().expect("travel thread").expect("travel");
    assert_eq!(landed.expect("reply")["ok"], true);
    assert_eq!(margins(&fake), hidden, "a travel drew under the hide");
    let show = RendererRequest::Show {
        holder: held.nonce().to_owned(),
    };
    drop(held);
    assert_eq!(served(&fixture, &show)["ok"], true);
    assert_eq!(
        margins(&fake),
        hidden + 1,
        "show did not draw the landed cursor"
    );
    let disable = RendererRequest::control(CursorOverlayControl::disable("fixture-session".into()));
    served(&fixture, &disable);
    renderer.join().expect("renderer").expect("clean exit");
}
