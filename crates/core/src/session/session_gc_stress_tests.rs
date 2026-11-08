use super::*;

/// Runs the spawn workload in its own process, so its children cannot hold
/// the locks of other tests in this one.
#[test]
fn gc_reaps_a_released_session_while_other_threads_spawn_children() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("session::gc_tests::stress_tests::subprocess_gc_spawn_stress")
        .env("AGENT_DESKTOP_GC_SPAWN_STRESS", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("1 passed"), "{stdout}");
}

#[test]
fn subprocess_gc_spawn_stress() {
    if std::env::var_os("AGENT_DESKTOP_GC_SPAWN_STRESS").is_none() {
        return;
    }
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let spawners: Vec<_> = (0..2)
        .map(|_| {
            let stop = stop.clone();
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = std::process::Command::new(std::env::current_exe().unwrap())
                        .arg("--exact")
                        .arg("session::gc_tests::subprocess_session_lease_holder")
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                }
            })
        })
        .collect();
    let reaped = (0..200)
        .filter(|_| {
            let _guard = HomeGuard::new();
            let mut manifest = start_session(StartSessionOptions {
                name: None,
                trace: SessionTraceMode::Off,
                ..Default::default()
            })
            .unwrap();
            manifest.created_at = 0;
            write_manifest(&manifest).unwrap();
            drop(acquire_liveness_lease(&manifest.id).unwrap().unwrap());
            gc_reaping(any_age, &[&manifest.id]).contains(&manifest.id)
        })
        .count();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    for spawner in spawners {
        spawner.join().unwrap();
    }
    assert_eq!(reaped, 200);
}
