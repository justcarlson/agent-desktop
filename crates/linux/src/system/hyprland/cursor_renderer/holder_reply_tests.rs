use agent_desktop_core::{CursorOverlayControl, CursorOverlayStyle};

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
