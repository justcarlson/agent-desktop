use super::{HOLDER_CAP_MS, HolderLease, expiry_for, sweep_abandoned};
use crate::system::hyprland::cursor_renderer::holder_file::{HolderFile, now_ms};
use crate::system::hyprland::cursor_renderer::holder_view::live_holders;
use crate::system::hyprland::cursor_renderer::registry_fixture::{
    RegistryFixture, finish, sleeper, start_time_of,
};

#[test]
fn a_lease_is_live_until_released_and_renewal_moves_its_expiry() {
    let fixture = RegistryFixture::new();
    let holders = &fixture.paths.holders;
    let mut lease = HolderLease::take(holders, now_ms() + 5_000).expect("take");
    let live = live_holders(holders);
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].nonce, lease.nonce());
    let later = lease.expiry() + 1_000;
    lease.renew(later).expect("renew");
    assert_eq!(live_holders(holders)[0].expiry, later);
    lease.release();
    assert!(live_holders(holders).is_empty());
}

#[test]
fn a_dropped_lease_removes_its_file() {
    let fixture = RegistryFixture::new();
    let holders = &fixture.paths.holders;
    drop(HolderLease::take(holders, now_ms() + 5_000).expect("take"));
    assert_eq!(std::fs::read_dir(holders).expect("dir").count(), 0);
}

#[test]
fn the_expiry_follows_the_work_and_never_passes_the_cap() {
    assert_eq!(expiry_for(1_000, 0, HOLDER_CAP_MS), 1_500);
    assert_eq!(expiry_for(60_000, 0, HOLDER_CAP_MS), HOLDER_CAP_MS);
}

#[test]
fn taking_a_lease_removes_only_abandoned_holders() {
    let fixture = RegistryFixture::new();
    let holders = &fixture.paths.holders;
    let child = sleeper();
    let dead_pid = child.id();
    let dead_start = start_time_of(dead_pid);
    finish(child);
    let own = std::process::id();
    let write = |nonce: &str, pid: u32, start_time: u64, expiry: u64| {
        HolderFile {
            pid,
            start_time,
            nonce: nonce.to_owned(),
            expiry,
        }
        .write_in(holders)
        .expect("write");
    };
    let now = now_ms();
    write("dead", dead_pid, dead_start, now + 60_000);
    write("long-expired", own, start_time_of(own), 1);
    write("just-expired", own, start_time_of(own), now - 1);
    write("other-live", own, start_time_of(own), now + 60_000);
    sweep_abandoned(holders, now);
    let mut left: Vec<String> = std::fs::read_dir(holders)
        .expect("dir")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    left.sort_unstable();
    assert_eq!(left, ["just-expired.json", "other-live.json"]);
}
