use std::time::{Duration, Instant};

use super::{hide_every, hide_one};
use crate::system::hyprland::cursor_renderer::fake_renderer::{FakeAnswer, FakeRenderer};
use crate::system::hyprland::cursor_renderer::hide_refusal::HideRefusal;
use crate::system::hyprland::cursor_renderer::registry_fixture::{
    RegistryFixture, finish, record_for, sleeper,
};
use crate::system::hyprland::cursor_renderer::renderer_paths::RendererPaths;
use crate::system::hyprland::cursor_renderer::renderer_record::RendererRecord;
use crate::system::hyprland::cursor_renderer::renderer_request::RendererRequest;

fn hide() -> RendererRequest {
    RendererRequest::Hide {
        holder: "h1".into(),
        expiry_ms: 1,
    }
}

fn soon(ms: u64) -> Instant {
    Instant::now() + Duration::from_millis(ms)
}

fn entry(fake: &FakeRenderer) -> (RendererPaths, RendererRecord) {
    let record = record_for(std::process::id(), None, "fake");
    (fake.paths.clone(), record)
}

#[test]
fn every_renderer_receives_the_hide_and_each_acknowledgement_counts() {
    let fixture = RegistryFixture::new();
    let first = FakeRenderer::start(
        &fixture.paths.dir,
        "1111111111111111",
        FakeAnswer::Acknowledge,
    );
    let second = FakeRenderer::start(
        &fixture.paths.dir,
        "2222222222222222",
        FakeAnswer::Acknowledge,
    );
    let acked = hide_every(&[entry(&first), entry(&second)], &hide(), soon(500)).expect("acked");
    assert_eq!(acked.len(), 2);
    assert_eq!(first.messages(), ["hide"]);
    assert_eq!(second.messages(), ["hide"]);
}

#[test]
fn a_withheld_acknowledgement_times_out_inside_the_deadline() {
    let fixture = RegistryFixture::new();
    let fake = FakeRenderer::start(&fixture.paths.dir, "3333333333333333", FakeAnswer::Withhold);
    let started = Instant::now();
    let refused = hide_every(&[entry(&fake)], &hide(), soon(200)).expect_err("withheld");
    assert!(
        matches!(refused, HideRefusal::Timeout { .. }),
        "{refused:?}"
    );
    assert!(started.elapsed() < Duration::from_millis(400));
}

#[test]
fn a_refusing_renderer_fails_the_hide() {
    let fixture = RegistryFixture::new();
    let fake = FakeRenderer::start(&fixture.paths.dir, "4444444444444444", FakeAnswer::Refuse);
    let refused = hide_every(&[entry(&fake)], &hide(), soon(500)).expect_err("refused");
    assert!(matches!(refused, HideRefusal::Failed { .. }), "{refused:?}");
}

#[test]
fn a_live_renderer_that_does_not_listen_yet_times_out_and_a_dead_one_counts_as_hidden() {
    let fixture = RegistryFixture::new();
    let child = sleeper();
    let paths = RendererPaths::in_dir(&fixture.paths.dir, "5555555555555555").expect("paths");
    let alive = record_for(child.id(), None, "starting");
    let refused = hide_one(&paths, &alive, &hide(), soon(100)).expect_err("still starting");
    assert!(
        matches!(refused, HideRefusal::Timeout { .. }),
        "{refused:?}"
    );
    finish(child);
    assert_eq!(hide_one(&paths, &alive, &hide(), soon(100)), Ok(false));
}
