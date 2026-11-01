//! A throwaway registry dir and live stand-in processes for tests.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::renderer_owner::RendererOwner;
use super::renderer_paths::RendererPaths;
use super::renderer_record::RendererRecord;
use crate::system::process::proc_stat::read_stat;

/// A private registry dir removed when the fixture drops.
pub(crate) struct RegistryFixture {
    pub(crate) paths: RendererPaths,
    root: PathBuf,
}

impl RegistryFixture {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = PathBuf::from(format!(
            "/tmp/ad-cr-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let paths = RendererPaths::for_session(&root, "fixture-signature", "fixture-session")
            .expect("fixture paths fit");
        paths.ensure_dirs().expect("fixture dirs");
        Self { paths, root }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for RegistryFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A child that sleeps until killed, standing in for a renderer process.
pub(crate) fn sleeper() -> Child {
    Command::new("sleep")
        .arg("60")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sleep")
}

/// A child that ignores SIGTERM and sleeps until killed.
pub(crate) fn term_ignoring_sleeper() -> Child {
    Command::new("sh")
        .args(["-c", "trap '' TERM; while :; do sleep 1; done"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sh")
}

pub(crate) fn start_time_of(pid: u32) -> u64 {
    read_stat(pid)
        .expect("stat readable")
        .expect("process alive")
        .start_time
}

/// A record naming `pid` with its real start time, or with `start_time`
/// when given.
pub(crate) fn record_for(pid: u32, start_time: Option<u64>, generation: &str) -> RendererRecord {
    RendererRecord {
        owner: RendererOwner {
            pid,
            start_time: start_time.unwrap_or_else(|| start_time_of(pid)),
            exe: std::fs::read_link(format!("/proc/{pid}/exe")).unwrap_or_default(),
            generation: generation.to_owned(),
        },
        protocol: "wayland-v1".into(),
        session_id: "fixture-session".into(),
        hypr_signature: "fixture-signature".into(),
        wayland_socket: PathBuf::from("/run/user/1000/hypr/fixture/wayland-1"),
        created_at: 1,
    }
}

/// Whether `child` is still running, without reaping it.
pub(crate) fn is_running(child: &Child) -> bool {
    read_stat(child.id()).ok().flatten().is_some_and(|stat| {
        !matches!(
            stat.state,
            crate::system::process::proc_state::ProcState::Zombie
                | crate::system::process::proc_state::ProcState::Dead
        )
    })
}

pub(crate) fn finish(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}
