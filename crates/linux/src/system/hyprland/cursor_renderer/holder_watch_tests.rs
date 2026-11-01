use agent_desktop_core::AdapterError;

use super::{HolderCanvas, HolderWatch};
use crate::system::hyprland::cursor_renderer::holder_file::now_ms;
use crate::system::hyprland::cursor_renderer::holder_lease::HolderLease;
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;

#[derive(Default)]
struct Canvas {
    calls: Vec<&'static str>,
}

impl HolderCanvas for Canvas {
    fn clear_all(&mut self) -> Result<(), AdapterError> {
        self.calls.push("clear");
        Ok(())
    }

    fn sync(&mut self) -> Result<(), AdapterError> {
        self.calls.push("sync");
        Ok(())
    }

    fn redraw(&mut self) -> Result<(), AdapterError> {
        self.calls.push("redraw");
        Ok(())
    }
}

fn lease(fixture: &RegistryFixture) -> HolderLease {
    HolderLease::take(&fixture.paths.holders, now_ms() + 5_000).expect("lease")
}

#[test]
fn hide_clears_then_syncs_before_it_answers() {
    let fixture = RegistryFixture::new();
    let held = lease(&fixture);
    let mut canvas = Canvas::default();
    let mut watch = HolderWatch::default();
    watch
        .hide(&fixture.paths.holders, held.nonce(), &mut canvas)
        .expect("hidden");
    assert_eq!(canvas.calls, ["clear", "sync"]);
    assert!(watch.is_suppressed());
}

#[test]
fn a_hide_for_a_holder_that_is_not_live_refuses_and_draws_nothing() {
    let fixture = RegistryFixture::new();
    let mut canvas = Canvas::default();
    let (kind, _) = HolderWatch::default()
        .hide(&fixture.paths.holders, "no-such-holder", &mut canvas)
        .expect_err("not live");
    assert_eq!(kind, "cursor_holder_not_live");
    assert!(canvas.calls.is_empty());
}

#[test]
fn show_from_one_holder_keeps_another_holders_suppression() {
    let fixture = RegistryFixture::new();
    let holders = &fixture.paths.holders;
    let mut first = lease(&fixture);
    let second = lease(&fixture);
    let mut canvas = Canvas::default();
    let mut watch = HolderWatch::default();
    watch
        .hide(holders, first.nonce(), &mut canvas)
        .expect("first");
    watch
        .hide(holders, second.nonce(), &mut canvas)
        .expect("second");
    canvas.calls.clear();
    first.release();
    watch.show(holders, &mut canvas).expect("first show");
    assert!(canvas.calls.is_empty());
    assert!(watch.is_suppressed());
    drop(second);
    watch.show(holders, &mut canvas).expect("second show");
    assert_eq!(canvas.calls, ["redraw"]);
    assert!(!watch.is_suppressed());
}

#[test]
fn a_renderer_started_during_a_hold_draws_nothing_until_the_holder_leaves() {
    let fixture = RegistryFixture::new();
    let holders = &fixture.paths.holders;
    let held = lease(&fixture);
    let mut watch = HolderWatch::loaded(holders);
    assert!(watch.is_suppressed());
    let mut canvas = Canvas::default();
    watch.refresh(holders, &mut canvas).expect("tick");
    assert!(canvas.calls.is_empty());
    drop(held);
    watch.refresh(holders, &mut canvas).expect("tick");
    assert_eq!(canvas.calls, ["sync", "redraw"]);
}

#[test]
fn a_holder_that_appears_between_ticks_clears_the_cursor() {
    let fixture = RegistryFixture::new();
    let holders = &fixture.paths.holders;
    let mut watch = HolderWatch::loaded(holders);
    assert!(!watch.is_suppressed());
    let mut canvas = Canvas::default();
    watch.refresh(holders, &mut canvas).expect("idle tick");
    assert!(canvas.calls.is_empty());
    let _held = lease(&fixture);
    watch.refresh(holders, &mut canvas).expect("tick");
    assert_eq!(canvas.calls, ["clear"]);
    watch.refresh(holders, &mut canvas).expect("tick");
    assert_eq!(canvas.calls, ["clear"]);
}
