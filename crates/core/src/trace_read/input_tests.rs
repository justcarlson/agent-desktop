use super::{ReadOptions, read_merged};
use std::fs;
use std::io::Write;

fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("trace-input-{}", crate::refs::new_snapshot_id()));
    fs::create_dir(&path).unwrap();
    path
}

#[test]
fn trace_input_budget_applies_before_the_output_tail_limit() {
    let dir = directory();
    let mut file = fs::File::create(dir.join("100-1.jsonl")).unwrap();
    for _ in 0..100_001 {
        writeln!(file, r#"{{"event":"a","ts_ms":1}}"#).unwrap();
    }
    drop(file);
    let result = read_merged(
        &dir,
        &ReadOptions {
            limit: 1,
            event_prefix: None,
        },
    );
    assert!(
        result.is_err(),
        "output limits must not allow unbounded input retention"
    );
}

#[test]
fn trace_byte_budget_is_shared_between_segments() {
    let dir = directory();
    for id in 1..=5 {
        let file = fs::File::create(dir.join(format!("{id}-1.jsonl"))).unwrap();
        file.set_len(8 * 1024 * 1024).unwrap();
    }
    let error = read_merged(&dir, &ReadOptions::default()).unwrap_err();
    assert!(error.to_string().contains("32 MiB"));
}

#[cfg(unix)]
#[test]
fn fifo_segment_is_rejected_instead_of_read_as_a_trace() {
    const CHILD: &str = "AGENT_DESKTOP_TRACE_FIFO_TEST";
    if std::env::var_os(CHILD).is_none() {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "trace_read::input_tests::fifo_segment_is_rejected_instead_of_read_as_a_trace",
            ])
            .env(CHILD, "1")
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("trace reader blocked on FIFO");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    let dir = directory();
    let path = dir.join("100-1.jsonl");
    let name = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let result = read_merged(&dir, &ReadOptions::default()).unwrap();
    assert!(result.segments.is_empty());
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.kind == super::TraceWarningKind::UnreadableSegment)
    );
}
