use super::{Identity, Liveness, identity_of, verdict};
use crate::system::proc_stat::{ProcStat, ProcState};

fn token(start: u64) -> String {
    format!("boot:{start}")
}

fn stat(state: ProcState, start_time: u64) -> Option<ProcStat> {
    Some(ProcStat { state, start_time })
}

#[test]
fn a_sleeping_process_with_the_expected_token_is_alive() {
    assert_eq!(
        verdict(stat(ProcState::Sleeping, 9), "boot:9", token),
        Liveness::Alive
    );
}

#[test]
fn a_zombie_is_gone_though_kill_zero_would_succeed() {
    assert_eq!(
        verdict(stat(ProcState::Zombie, 9), "boot:9", token),
        Liveness::Gone
    );
    assert_eq!(
        verdict(stat(ProcState::Dead, 9), "boot:9", token),
        Liveness::Gone
    );
}

#[test]
fn a_missing_entry_is_gone() {
    assert_eq!(verdict(None, "boot:9", token), Liveness::Gone);
}

#[test]
fn a_token_that_differs_is_gone_even_for_a_live_process() {
    assert_eq!(
        verdict(stat(ProcState::Running, 10), "boot:9", token),
        Liveness::Gone
    );
    assert_eq!(
        verdict(stat(ProcState::Running, 9), "other:9", token),
        Liveness::Gone
    );
}

#[test]
fn a_process_stopped_by_signal_or_tracer_is_stopped() {
    assert_eq!(
        verdict(stat(ProcState::Stopped, 9), "boot:9", token),
        Liveness::Stopped
    );
}

#[test]
fn an_unknown_state_is_alive_not_gone() {
    assert_eq!(
        verdict(stat(ProcState::Other, 9), "boot:9", token),
        Liveness::Alive
    );
}

#[test]
fn identity_keeps_a_replaced_process_apart_from_one_that_left() {
    let same = identity_of(Ok(stat(ProcState::Sleeping, 9)), "boot:9", token);
    let replaced = identity_of(Ok(stat(ProcState::Sleeping, 10)), "boot:9", token);
    let left = identity_of(Ok(None), "boot:9", token);
    let zombie = identity_of(Ok(stat(ProcState::Zombie, 9)), "boot:9", token);
    let unread = identity_of(Err(()), "boot:9", token);
    assert_eq!(same, Identity::Same);
    assert_eq!(replaced, Identity::Replaced);
    assert_eq!(left, Identity::Gone);
    assert_eq!(zombie, Identity::Gone);
    assert_eq!(unread, Identity::Unreadable);
}

#[cfg(target_os = "linux")]
mod live {
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    use super::super::{Liveness, liveness};
    use crate::system::process_facts;

    fn spawn_sleep() -> Child {
        Command::new("sleep")
            .arg("60")
            .stdin(Stdio::null())
            .spawn()
            .expect("sleep spawns")
    }

    fn token_of(pid: u32) -> String {
        process_facts::read(pid)
            .expect("the child is running")
            .instance
    }

    fn wait_for(pid: u32, token: &str, wanted: Liveness) -> Liveness {
        let started = Instant::now();
        loop {
            let seen = liveness(pid, token);
            if seen == wanted || started.elapsed() > Duration::from_secs(5) {
                return seen;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_spawned_sleep_is_alive() {
        let mut child = spawn_sleep();
        let token = token_of(child.id());
        assert_eq!(liveness(child.id(), &token), Liveness::Alive);
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn an_exited_child_nobody_reaped_is_gone() {
        let mut child = spawn_sleep();
        let pid = child.id();
        let token = token_of(pid);
        Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status()
            .expect("kill runs");
        assert_eq!(wait_for(pid, &token, Liveness::Gone), Liveness::Gone);
        assert!(
            std::path::Path::new(&format!("/proc/{pid}")).exists(),
            "the child is a zombie, not yet reaped"
        );
        let _ = child.wait();
    }

    #[test]
    fn a_stopped_child_is_stopped_and_a_continued_one_alive() {
        let mut child = spawn_sleep();
        let pid = child.id();
        let token = token_of(pid);
        Command::new("kill")
            .args(["-STOP", &pid.to_string()])
            .status()
            .expect("kill runs");
        assert_eq!(wait_for(pid, &token, Liveness::Stopped), Liveness::Stopped);
        Command::new("kill")
            .args(["-CONT", &pid.to_string()])
            .status()
            .expect("kill runs");
        assert_eq!(wait_for(pid, &token, Liveness::Alive), Liveness::Alive);
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn the_token_of_another_run_makes_a_live_pid_gone() {
        let mut child = spawn_sleep();
        assert_eq!(liveness(child.id(), "0000:1"), Liveness::Gone);
        let _ = child.kill();
        let _ = child.wait();
    }
}
