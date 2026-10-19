use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use agent_desktop_core::{ImageBuffer, ImageFormat};

use super::*;

static NEXT: AtomicU32 = AtomicU32::new(0);

struct Tools(PathBuf);

impl Tools {
    fn with(names: &[&str]) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "ad-wl-tools-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).expect("a tools directory");
        for name in names {
            let path = dir.join(name);
            fs::write(&path, "#!/bin/sh\n").expect("a tool");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
        }
        Self(dir)
    }

    fn path(&self) -> Option<OsString> {
        Some(self.0.clone().into_os_string())
    }
}

impl Drop for Tools {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Script {
    replies: RefCell<VecDeque<RunOutcome>>,
    seen: RefCell<Vec<RunSpec>>,
}

impl Script {
    fn new(replies: Vec<RunOutcome>) -> Self {
        Self {
            replies: RefCell::new(replies.into()),
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl CommandRunner for Script {
    fn run(&self, spec: &RunSpec, _deadline: Deadline) -> RunOutcome {
        self.seen.borrow_mut().push(spec.clone());
        self.replies
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| RunOutcome::NotStarted("no scripted reply".into()))
    }
}

fn ok(stdout: &[u8]) -> RunOutcome {
    RunOutcome::Exited {
        code: Some(0),
        stdout: stdout.to_vec(),
        stderr: String::new(),
    }
}

fn fails(stderr: &str) -> RunOutcome {
    RunOutcome::Exited {
        code: Some(1),
        stdout: Vec::new(),
        stderr: stderr.into(),
    }
}

fn deadline() -> Deadline {
    Deadline::after(5_000).expect("a deadline")
}

fn lease() -> InteractionLease {
    InteractionLease::guarded(deadline(), ()).expect("a lease")
}

fn read_via(
    script: &Script,
    tools: &Tools,
    format: ClipboardFormat,
) -> Result<Option<ClipboardContent>, AdapterError> {
    Wayland {
        runner: script,
        search_path: tools.path(),
    }
    .read(format, deadline())
}

fn set_via(script: &Script, tools: &Tools, content: &ClipboardContent) -> Result<(), AdapterError> {
    Wayland {
        runner: script,
        search_path: tools.path(),
    }
    .set(content, &lease())
}

fn clear_via(script: &Script, tools: &Tools) -> Result<(), AdapterError> {
    Wayland {
        runner: script,
        search_path: tools.path(),
    }
    .clear(&lease())
}

fn both() -> Tools {
    Tools::with(&[COPY, PASTE])
}

fn png() -> Vec<u8> {
    let mut data = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&2u32.to_be_bytes());
    data.extend_from_slice(&3u32.to_be_bytes());
    data.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    data
}

const TEXT_LIST: &[u8] = b"UTF8_STRING\nSTRING\nTEXT\ntext/plain;charset=utf-8\ntext/plain\n";

#[test]
fn nothing_copied_is_an_absent_value_and_not_an_error() {
    let script = Script::new(vec![fails("Nothing is copied\n")]);
    let outcome = read_via(&script, &both(), ClipboardFormat::Auto);
    assert!(outcome.expect("an empty clipboard is no error").is_none());
}

#[test]
fn text_is_read_with_the_type_the_owner_offered() {
    let script = Script::new(vec![ok(TEXT_LIST), ok("héllo".as_bytes())]);
    let content = read_via(&script, &both(), ClipboardFormat::Text).expect("a read");
    assert!(matches!(content, Some(ClipboardContent::Text(ref text)) if text == "héllo"));
    let seen = script.seen.borrow();
    assert_eq!(seen[0].args, ["--list-types"]);
    assert_eq!(seen[1].args, ["--no-newline", "--type", "UTF8_STRING"]);
}

#[test]
fn an_image_and_a_file_list_are_read_by_their_own_types() {
    let image = Script::new(vec![ok(b"image/png\ntext/plain\n"), ok(&png())]);
    let content = read_via(&image, &both(), ClipboardFormat::Image).expect("a read");
    assert!(
        matches!(content, Some(ClipboardContent::Image(ref picture)) if picture.width == 2 && picture.height == 3)
    );
    let files = Script::new(vec![ok(b"text/uri-list\n"), ok(b"file:///tmp/a%20b\r\n")]);
    let content = read_via(&files, &both(), ClipboardFormat::FileUrls).expect("a read");
    assert!(
        matches!(content, Some(ClipboardContent::FileUrls(ref paths)) if paths == &["/tmp/a b"])
    );
}

#[test]
fn a_format_the_owner_does_not_offer_is_absent() {
    let script = Script::new(vec![ok(b"text/plain\n")]);
    let outcome = read_via(&script, &both(), ClipboardFormat::Image);
    assert!(outcome.expect("no error").is_none());
    let refused = Script::new(vec![
        ok(b"image/png\n"),
        fails("Clipboard content is not available as requested type \"image/png\"\n"),
    ]);
    let outcome = read_via(&refused, &both(), ClipboardFormat::Image);
    assert!(outcome.expect("no error").is_none());
}

#[test]
fn a_seat_with_no_keyboard_is_a_distinct_error() {
    let script = Script::new(vec![fails("This seat has no keyboard\n")]);
    let error = read_via(&script, &both(), ClipboardFormat::Text).expect_err("an error");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(
        error.details.expect("details")["reason"],
        "no_keyboard_seat"
    );
}

#[test]
fn a_compositor_that_never_answers_is_a_timeout() {
    let script = Script::new(vec![RunOutcome::TimedOut]);
    let error = read_via(&script, &both(), ClipboardFormat::Text).expect_err("an error");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn output_over_the_cap_is_an_action_failure() {
    let script = Script::new(vec![ok(b"image/png\n"), RunOutcome::OverCap]);
    let error = read_via(&script, &both(), ClipboardFormat::Image).expect_err("an error");
    assert_eq!(error.code, ErrorCode::ActionFailed);
}

#[test]
fn a_missing_wl_paste_names_the_package() {
    let script = Script::new(Vec::new());
    let error =
        read_via(&script, &Tools::with(&[COPY]), ClipboardFormat::Text).expect_err("an error");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(
        error
            .suggestion
            .expect("a suggestion")
            .contains("wl-clipboard")
    );
    assert!(script.seen.borrow().is_empty());
}

#[test]
fn a_relative_search_path_entry_never_supplies_a_tool() {
    let tools = Tools::with(&[PASTE]);
    assert!(locate(PASTE, tools.path().as_deref()).is_some());
    assert!(locate(PASTE, Some(OsStr::new("relative/dir:."))).is_none());
    assert!(locate(PASTE, None).is_none());
}

#[test]
fn a_text_set_sends_the_payload_on_stdin_and_not_in_the_arguments() {
    let script = Script::new(vec![ok(b""), ok(TEXT_LIST)]);
    set_via(
        &script,
        &both(),
        &ClipboardContent::Text("s3cret-token".into()),
    )
    .expect("a set");
    let seen = script.seen.borrow();
    assert_eq!(seen[0].args, ["--type", "text/plain;charset=utf-8"]);
    assert_eq!(seen[0].stdin.as_deref(), Some(b"s3cret-token".as_slice()));
    assert!(!seen[0].capture_stdout);
    assert!(seen[0].args.iter().all(|arg| !arg.contains("s3cret")));
}

#[test]
fn a_file_set_offers_the_uri_list_and_no_second_type() {
    let script = Script::new(vec![ok(b""), ok(b"text/uri-list\n")]);
    let content = ClipboardContent::FileUrls(vec!["/tmp/a b".into()]);
    set_via(&script, &both(), &content).expect("a set");
    let seen = script.seen.borrow();
    assert_eq!(seen[0].args, ["--type", "text/uri-list"]);
    assert_eq!(
        seen[0].stdin.as_deref(),
        Some(b"file:///tmp/a%20b\r\n".as_slice())
    );
}

#[test]
fn an_image_set_offers_the_png() {
    let script = Script::new(vec![ok(b""), ok(b"image/png\n")]);
    let content = ClipboardContent::Image(ImageBuffer {
        data: png(),
        format: ImageFormat::Png,
        width: 2,
        height: 3,
        scale_factor: 1.0,
    });
    set_via(&script, &both(), &content).expect("a set");
    assert_eq!(script.seen.borrow()[0].args, ["--type", "image/png"]);
}

#[test]
fn clear_runs_wl_copy_clear_and_checks_the_clipboard_is_empty() {
    let script = Script::new(vec![ok(b""), fails("Nothing is copied\n")]);
    clear_via(&script, &both()).expect("a clear");
    let seen = script.seen.borrow();
    assert_eq!(seen[0].args, ["--clear"]);
    assert_eq!(seen[0].stdin, None);
}

#[test]
fn a_clear_that_leaves_content_behind_is_unverified() {
    let script = Script::new(vec![ok(b""), ok(TEXT_LIST)]);
    let error = clear_via(&script, &both()).expect_err("an error");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn a_set_the_clipboard_does_not_show_is_unverified() {
    let script = Script::new(vec![ok(b""), ok(b"text/uri-list\n")]);
    let error =
        set_via(&script, &both(), &ClipboardContent::Text("x".into())).expect_err("an error");
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn a_set_that_times_out_is_uncertain_and_never_not_delivered() {
    let script = Script::new(vec![RunOutcome::TimedOut]);
    let error =
        set_via(&script, &both(), &ClipboardContent::Text("x".into())).expect_err("an error");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
}

#[test]
fn a_set_ended_by_a_signal_or_an_unknown_failure_is_uncertain() {
    let signalled = Script::new(vec![RunOutcome::Exited {
        code: None,
        stdout: Vec::new(),
        stderr: String::new(),
    }]);
    let error =
        set_via(&signalled, &both(), &ClipboardContent::Text("x".into())).expect_err("an error");
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    let unknown = Script::new(vec![fails("surprise\n")]);
    let error =
        set_via(&unknown, &both(), &ClipboardContent::Text("x".into())).expect_err("an error");
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
}

#[test]
fn a_set_refused_for_a_known_cause_changed_nothing() {
    let script = Script::new(vec![fails("This seat has no keyboard\n")]);
    let error =
        set_via(&script, &both(), &ClipboardContent::Text("x".into())).expect_err("an error");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let start = Script::new(vec![RunOutcome::NotStarted("denied".into())]);
    let error =
        set_via(&start, &both(), &ClipboardContent::Text("x".into())).expect_err("an error");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn a_set_needs_both_tools_before_it_writes() {
    let script = Script::new(Vec::new());
    let error = set_via(
        &script,
        &Tools::with(&[COPY]),
        &ClipboardContent::Text("x".into()),
    )
    .expect_err("an error");
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(script.seen.borrow().is_empty());
}
