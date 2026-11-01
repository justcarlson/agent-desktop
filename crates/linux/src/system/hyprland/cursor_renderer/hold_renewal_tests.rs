use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{HoldRenewal, RenewTimings, renew_once};
use crate::system::hyprland::cursor_renderer::hold_registry::{
    ensure_held, is_held, publish, withdraw,
};
use crate::system::hyprland::cursor_renderer::holder_file::now_ms;
use crate::system::hyprland::cursor_renderer::holder_lease::HolderLease;
use crate::system::hyprland::cursor_renderer::holder_view::live_holders;
use crate::system::hyprland::cursor_renderer::registry_fixture::RegistryFixture;

const QUICK: RenewTimings = RenewTimings {
    cap_ms: 400,
    margin_ms: 300,
};

#[test]
fn a_holder_near_expiry_during_long_work_is_renewed() {
    let fixture = RegistryFixture::new();
    let holders = &fixture.paths.holders;
    let first = now_ms() + 400;
    let lease = Arc::new(Mutex::new(HolderLease::take(holders, first).expect("take")));
    let mut renewal = HoldRenewal::start(Arc::clone(&lease), now_ms() + 5_000, QUICK);
    let until = Instant::now() + Duration::from_secs(3);
    while live_holders(holders)
        .first()
        .is_none_or(|holder| holder.expiry <= first)
    {
        assert!(Instant::now() < until, "the holder was not renewed");
        std::thread::sleep(Duration::from_millis(20));
    }
    renewal.stop();
    assert!(lease.lock().expect("lease").expiry() > first);
}

#[test]
fn renewal_stops_once_the_work_end_is_covered() {
    let fixture = RegistryFixture::new();
    let work_end = now_ms() + 100;
    let lease =
        Mutex::new(HolderLease::take(&fixture.paths.holders, work_end + 500).expect("take"));
    assert!(!renew_once(&lease, work_end, QUICK));
    assert_eq!(lease.lock().expect("lease").expiry(), work_end + 500);
}

#[test]
fn a_failed_renewal_loses_the_hold_and_stops_the_next_event() {
    let fixture = RegistryFixture::new();
    let lease =
        Mutex::new(HolderLease::take(&fixture.paths.holders, now_ms() + 300).expect("take"));
    let nonce = lease.lock().expect("lease").nonce().to_owned();
    publish(&nonce, vec![]);
    std::fs::remove_dir_all(&fixture.paths.holders).expect("remove holders");
    assert!(!renew_once(&lease, now_ms() + 5_000, QUICK));
    assert!(!is_held(&nonce));
    let error = ensure_held().expect_err("a lost hold stops the work");
    assert_eq!(
        error.details.expect("details")["kind"],
        "cursor_hide_renewal_failed"
    );
    withdraw(&nonce);
    assert!(ensure_held().is_ok());
}

#[test]
fn a_renewal_after_the_expiry_loses_the_hold() {
    let fixture = RegistryFixture::new();
    let lease = Mutex::new(HolderLease::take(&fixture.paths.holders, now_ms() - 1).expect("take"));
    let nonce = lease.lock().expect("lease").nonce().to_owned();
    publish(&nonce, vec![]);
    assert!(!renew_once(&lease, now_ms() + 5_000, QUICK));
    assert!(!is_held(&nonce));
    withdraw(&nonce);
}
