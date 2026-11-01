use agent_desktop_core::{AdapterError, Deadline, ErrorCode, Rect};

use super::HideHold;
use crate::system::hyprland::cursor_renderer::fake_renderer::{FakeAnswer, FakeRenderer};
use crate::system::hyprland::cursor_renderer::hold_registry::acknowledged_renderers;
use crate::system::hyprland::cursor_renderer::hold_renewal::RenewTimings;
use crate::system::hyprland::cursor_renderer::holder_view::{any_live, live_holders};
use crate::system::hyprland::cursor_renderer::registry_fixture::{
    RegistryFixture, finish, record_for, sleeper,
};
use crate::system::hyprland::cursor_renderer::renderer_paths::RendererPaths;
use crate::system::hyprland::receiver::layer_info::LayerInfo;

fn cursor_layer(pid: u32) -> LayerInfo {
    LayerInfo {
        namespace: "agent-desktop-cursor".into(),
        pid: Some(pid),
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 32.0,
            height: 32.0,
        },
        level: 3,
        output: "DP-1".into(),
    }
}

fn soon() -> Deadline {
    Deadline::after(2_000).expect("deadline")
}

fn kind(error: &AdapterError) -> &str {
    error.details.as_ref().expect("details")["kind"]
        .as_str()
        .expect("kind")
}

fn take(fixture: &RegistryFixture, layers: Vec<LayerInfo>) -> Result<HideHold, AdapterError> {
    HideHold::take(
        &fixture.paths.dir,
        soon(),
        RenewTimings::STANDARD,
        &mut |_| Ok(layers.clone()),
    )
}

#[test]
fn a_hold_hides_every_renderer_and_its_release_shows_them() {
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
    let mut hold = take(&fixture, vec![cursor_layer(std::process::id())]).expect("hidden");
    assert!(any_live(&fixture.paths.holders));
    assert!(acknowledged_renderers().contains(&std::process::id()));
    assert_eq!(first.messages(), ["hide"]);
    assert_eq!(second.messages(), ["hide"]);
    hold.release();
    assert!(!any_live(&fixture.paths.holders));
    assert_eq!(first.messages(), ["hide", "show"]);
    assert_eq!(second.messages(), ["hide", "show"]);
}

#[test]
fn a_withheld_hide_acknowledgement_refuses_and_leaves_nothing_hidden() {
    let fixture = RegistryFixture::new();
    let _fake = FakeRenderer::start(&fixture.paths.dir, "3333333333333333", FakeAnswer::Withhold);
    let error = take(&fixture, Vec::new()).err().expect("withheld");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(kind(&error), "cursor_hide_timeout");
    assert!(!any_live(&fixture.paths.holders));
}

#[test]
fn a_foreign_layer_with_the_cursor_namespace_refuses() {
    let fixture = RegistryFixture::new();
    let error = take(&fixture, vec![cursor_layer(1)])
        .err()
        .expect("foreign layer");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(kind(&error), "cursor_unknown_overlay");
    assert!(!any_live(&fixture.paths.holders));
}

#[test]
fn another_builds_live_renderer_refuses() {
    let fixture = RegistryFixture::new();
    let child = sleeper();
    let paths = RendererPaths::in_dir(&fixture.paths.dir, "4444444444444444").expect("paths");
    record_for(child.id(), None, "other")
        .write_atomic(&paths.record)
        .expect("record");
    let error = take(&fixture, Vec::new()).err().expect("incompatible");
    assert_eq!(kind(&error), "cursor_renderer_incompatible");
    finish(child);
}

#[test]
fn an_unreadable_layer_stack_fails_the_hide() {
    let fixture = RegistryFixture::new();
    let error = HideHold::take(
        &fixture.paths.dir,
        soon(),
        RenewTimings::STANDARD,
        &mut |_| Err(AdapterError::internal("no layers")),
    )
    .err()
    .expect("no layers");
    assert_eq!(kind(&error), "cursor_hide_failed");
}

#[test]
fn a_renderer_that_starts_after_the_first_round_is_hidden_late() {
    let fixture = RegistryFixture::new();
    let mut late = None;
    let mut read_layers = |_| {
        late = Some(FakeRenderer::start(
            &fixture.paths.dir,
            "5555555555555555",
            FakeAnswer::Acknowledge,
        ));
        Ok(vec![cursor_layer(std::process::id())])
    };
    let hold = HideHold::take(
        &fixture.paths.dir,
        soon(),
        RenewTimings::STANDARD,
        &mut read_layers,
    )
    .expect("hidden late");
    assert_eq!(late.as_ref().expect("late renderer").messages(), ["hide"]);
    drop(hold);
}

#[test]
fn releasing_one_hold_keeps_another_holders_suppression() {
    let fixture = RegistryFixture::new();
    let fake = FakeRenderer::start(
        &fixture.paths.dir,
        "6666666666666666",
        FakeAnswer::Acknowledge,
    );
    let mut first = take(&fixture, Vec::new()).expect("first");
    let second = take(&fixture, Vec::new()).expect("second");
    first.release();
    assert_eq!(live_holders(&fixture.paths.holders).len(), 1);
    assert_eq!(fake.messages(), ["hide", "hide", "show"]);
    drop(second);
    assert!(!any_live(&fixture.paths.holders));
}

#[test]
fn work_longer_than_the_cap_renews_its_holder() {
    let fixture = RegistryFixture::new();
    let quick = RenewTimings {
        cap_ms: 200,
        margin_ms: 100,
    };
    let hold =
        HideHold::take(&fixture.paths.dir, soon(), quick, &mut |_| Ok(Vec::new())).expect("hold");
    assert!(hold.renewal.is_some());
    let short = HideHold::take(
        &fixture.paths.dir,
        Deadline::after(50).expect("deadline"),
        quick,
        &mut |_| Ok(Vec::new()),
    )
    .expect("short hold");
    assert!(short.renewal.is_none());
}
