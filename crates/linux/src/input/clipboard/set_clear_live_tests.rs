use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread::sleep;
use std::time::{Duration, Instant};

use agent_desktop_core::{
    ClipboardContent, ClipboardFormat, Deadline, ImageBuffer, ImageFormat, InputOps,
    InteractionLease,
};

use crate::LinuxAdapter;
use crate::input::clipboard::owner_locate::HELPER_NAME;
use crate::input::clipboard::owner_record::{self, OwnerRecord};

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

struct Session {
    _guard: std::sync::MutexGuard<'static, ()>,
    display: String,
}

fn private_session() -> Session {
    let guard = ONE_AT_A_TIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let private = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION")
        .expect("run inside scripts/private-session.sh: the live clipboard is never touched");
    let runtime = std::env::var("XDG_RUNTIME_DIR").expect("a private runtime directory");
    assert_eq!(
        runtime, private,
        "the runtime directory must be the private one"
    );
    let display = std::env::var("DISPLAY").expect("the private display");
    assert!(
        std::env::var_os("WAYLAND_DISPLAY").is_none(),
        "a private X11 session carries no Wayland display"
    );
    let home = std::env::var("AGENT_DESKTOP_HOME").expect("AGENT_DESKTOP_HOME in the private dir");
    assert!(
        Path::new(&home).starts_with(&private),
        "the state root must sit inside the private session, not {home}"
    );
    Session {
        _guard: guard,
        display,
    }
}

fn lease() -> InteractionLease {
    InteractionLease::guarded(Deadline::after(20_000).expect("a deadline"), ()).expect("a lease")
}

fn deadline() -> Deadline {
    Deadline::after(20_000).expect("a deadline")
}

fn built_helper() -> PathBuf {
    let exe = std::env::current_exe().expect("the test image");
    let debug_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("the target directory")
        .to_path_buf();
    let built = debug_dir.join(HELPER_NAME);
    assert!(
        built.is_file(),
        "build the helper first: cargo build -p agent-desktop-linux --bin {HELPER_NAME}"
    );
    built
}

fn install_helper_beside_the_image() {
    let beside = std::env::current_exe()
        .expect("the test image")
        .parent()
        .expect("its directory")
        .join(HELPER_NAME);
    let staging = beside.with_extension(format!("new{}", std::process::id()));
    fs::copy(built_helper(), &staging).expect("copy the helper");
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o755)).expect("executable");
    fs::rename(&staging, &beside).expect("install the helper");
}

const READER: &str = r#"
import sys
import gi
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, Gtk
board = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
data = board.wait_for_contents(Gdk.Atom.intern(sys.argv[1], False))
if data is None:
    sys.exit(3)
sys.stdout.buffer.write(bytes(data.get_data()))
"#;

const TAKER: &str = r#"
import sys
import gi
gi.require_version("Gtk", "3.0")
gi.require_version("Gdk", "3.0")
from gi.repository import Gdk, Gtk
board = Gtk.Clipboard.get(Gdk.SELECTION_CLIPBOARD)
board.set_text("theirs", -1)
while Gtk.events_pending():
    Gtk.main_iteration()
print("ready", flush=True)
Gtk.main()
"#;

fn script(name: &str, source: &str) -> PathBuf {
    let dir = PathBuf::from(std::env::var("XDG_RUNTIME_DIR").expect("the private directory"));
    let path = dir.join(format!("live-{name}.py"));
    fs::write(&path, source).expect("a script");
    path
}

fn foreign_read(display: &str, target: &str) -> Option<Vec<u8>> {
    let output = Command::new("python3")
        .arg("-I")
        .arg(script("reader", READER))
        .arg(target)
        .env("DISPLAY", display)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("python3 runs");
    output.status.success().then_some(output.stdout)
}

fn records(display: &str) -> Vec<OwnerRecord> {
    owner_record::record_dir()
        .map(|dir| owner_record::records_in(&dir, display))
        .unwrap_or_default()
}

fn the_record(display: &str) -> OwnerRecord {
    let mut found = records(display);
    assert_eq!(found.len(), 1, "exactly one owner record: {found:?}");
    found.remove(0)
}

fn gone(pid: u32) -> bool {
    let path = format!("/proc/{pid}/stat");
    match fs::read_to_string(path) {
        Err(_) => true,
        Ok(stat) => stat
            .rsplit_once(") ")
            .is_some_and(|(_, rest)| rest.starts_with('Z')),
    }
}

fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timed out: {what}"
        );
        sleep(Duration::from_millis(50));
    }
}

fn set(content: &ClipboardContent) {
    LinuxAdapter::new()
        .set_clipboard_content(content, &lease())
        .expect("the clipboard is set");
}

fn get(format: ClipboardFormat) -> Option<ClipboardContent> {
    LinuxAdapter::new()
        .get_clipboard_content(format, deadline())
        .expect("the clipboard is read")
}

fn clear_and_finish() {
    LinuxAdapter::new()
        .clear_clipboard(&lease())
        .expect("the clipboard is cleared");
}

fn text_of(content: Option<ClipboardContent>) -> Option<String> {
    match content {
        Some(ClipboardContent::Text(text)) => Some(text),
        _ => None,
    }
}

fn tiny_png() -> Vec<u8> {
    let mut data = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&3u32.to_be_bytes());
    data.extend_from_slice(&2u32.to_be_bytes());
    data.extend_from_slice(&[8, 2, 0, 0, 0]);
    data.extend_from_slice(&[0; 4]);
    data
}

#[test]
#[ignore = "needs a private X11 session and the built clipboard owner"]
fn text_set_is_served_by_a_detached_owner_and_read_by_a_second_process() {
    let session = private_session();
    install_helper_beside_the_image();
    set(&ClipboardContent::Text("live text value".into()));
    let owner = the_record(&session.display);
    assert!(owner_record::is_live_with(
        &owner,
        owner_record::start_ticks_of
    ));
    for target in [
        "UTF8_STRING",
        "text/plain;charset=utf-8",
        "text/plain",
        "STRING",
    ] {
        assert_eq!(
            foreign_read(&session.display, target).as_deref(),
            Some(b"live text value".as_slice()),
            "{target}"
        );
    }
    for descriptor in 0..3 {
        let link = fs::read_link(format!("/proc/{}/fd/{descriptor}", owner.pid)).expect("a link");
        assert_eq!(link, Path::new("/dev/null"), "descriptor {descriptor}");
    }
    let stat = fs::read_to_string(format!("/proc/{}/stat", owner.pid)).expect("stat");
    let session_id = stat
        .rsplit_once(") ")
        .and_then(|(_, rest)| rest.split_whitespace().nth(3))
        .and_then(|field| field.parse::<u32>().ok());
    assert_eq!(
        session_id,
        Some(owner.pid),
        "the owner leads its own session"
    );
    let environment = fs::read(format!("/proc/{}/environ", owner.pid)).expect("environ");
    let names: Vec<String> = environment
        .split(|byte| *byte == 0)
        .filter_map(|entry| String::from_utf8(entry.to_vec()).ok())
        .filter_map(|entry| entry.split_once('=').map(|(name, _)| name.to_owned()))
        .collect();
    assert!(!names.iter().any(|name| name == "PATH"), "{names:?}");
    assert!(
        !names.iter().any(|name| name.contains("LEASE")),
        "{names:?}"
    );
    assert_eq!(
        text_of(get(ClipboardFormat::Text)),
        Some("live text value".into())
    );
    clear_and_finish();
}

#[test]
#[ignore = "needs a private X11 session and the built CLI and clipboard owner"]
fn the_cli_exits_with_its_stdout_closed_while_the_owner_keeps_serving() {
    let session = private_session();
    let helper = built_helper();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).expect("executable");
    let cli = helper.with_file_name("agent-desktop");
    assert!(
        cli.is_file(),
        "build the CLI first: cargo build -p agent-desktop"
    );
    let started = Instant::now();
    let output = Command::new(&cli)
        .args(["clipboard-set", "from the cli"])
        .stdin(Stdio::null())
        .output()
        .expect("the CLI runs and its stdout reaches end-of-file");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(started.elapsed() < Duration::from_secs(15));
    assert_eq!(
        foreign_read(&session.display, "UTF8_STRING").as_deref(),
        Some(b"from the cli".as_slice())
    );
    let owner = the_record(&session.display);
    assert!(owner_record::is_live_with(
        &owner,
        owner_record::start_ticks_of
    ));
    let cleared = Command::new(&cli)
        .arg("clipboard-clear")
        .stdin(Stdio::null())
        .output()
        .expect("the CLI clears");
    assert!(cleared.status.success());
    wait_until("the owner ends", || gone(owner.pid));
}

#[test]
#[ignore = "needs a private X11 session and the built clipboard owner"]
fn clear_leaves_the_selection_unowned_and_ends_the_recorded_owner() {
    let session = private_session();
    install_helper_beside_the_image();
    set(&ClipboardContent::Text("to be cleared".into()));
    let owner = the_record(&session.display);
    clear_and_finish();
    assert!(get(ClipboardFormat::Auto).is_none());
    assert!(foreign_read(&session.display, "UTF8_STRING").is_none());
    wait_until("the owner ends", || gone(owner.pid));
    assert!(records(&session.display).is_empty());
}

#[test]
#[ignore = "needs a private X11 session and the built clipboard owner"]
fn a_second_set_replaces_the_first_owner_and_leaves_one_record() {
    let session = private_session();
    install_helper_beside_the_image();
    set(&ClipboardContent::Text("first".into()));
    let first = the_record(&session.display);
    set(&ClipboardContent::Text("second".into()));
    wait_until("the first owner ends", || gone(first.pid));
    let second = the_record(&session.display);
    assert_ne!(first.pid, second.pid);
    assert_eq!(
        foreign_read(&session.display, "UTF8_STRING").as_deref(),
        Some(b"second".as_slice())
    );
    clear_and_finish();
}

#[test]
#[ignore = "needs a private X11 session and the built clipboard owner"]
fn a_text_above_the_incr_threshold_round_trips_through_both_readers() {
    let session = private_session();
    install_helper_beside_the_image();
    let big: String = (0..3 * 1024 * 1024u32)
        .map(|n| char::from(b'a' + (n % 26) as u8))
        .collect();
    set(&ClipboardContent::Text(big.clone()));
    let foreign = foreign_read(&session.display, "UTF8_STRING").expect("a foreign read");
    assert!(
        foreign == big.as_bytes(),
        "the foreign reader must get every byte"
    );
    let ours = text_of(get(ClipboardFormat::Text)).expect("our read");
    assert!(ours == big, "our INCR reader must get every byte");
    clear_and_finish();
}

#[test]
#[ignore = "needs a private X11 session and the built clipboard owner"]
fn an_image_is_served_as_png_and_read_back() {
    let session = private_session();
    install_helper_beside_the_image();
    let png = tiny_png();
    set(&ClipboardContent::Image(ImageBuffer {
        data: png.clone(),
        format: ImageFormat::Png,
        width: 3,
        height: 2,
        scale_factor: 1.0,
    }));
    assert_eq!(
        foreign_read(&session.display, "image/png"),
        Some(png.clone())
    );
    let Some(ClipboardContent::Image(image)) = get(ClipboardFormat::Image) else {
        panic!("an image is read as an image")
    };
    assert_eq!((image.width, image.height, image.data), (3, 2, png));
    assert!(get(ClipboardFormat::Text).is_none());
    clear_and_finish();
}

#[test]
#[ignore = "needs a private X11 session and the built clipboard owner"]
fn file_paths_are_served_under_both_file_manager_targets() {
    let session = private_session();
    install_helper_beside_the_image();
    set(&ClipboardContent::FileUrls(vec![
        "/tmp/a b".into(),
        "/tmp/c".into(),
    ]));
    assert_eq!(
        foreign_read(&session.display, "text/uri-list").as_deref(),
        Some(b"file:///tmp/a%20b\r\nfile:///tmp/c\r\n".as_slice())
    );
    assert_eq!(
        foreign_read(&session.display, "x-special/gnome-copied-files").as_deref(),
        Some(b"copy\nfile:///tmp/a%20b\nfile:///tmp/c".as_slice())
    );
    let Some(ClipboardContent::FileUrls(paths)) = get(ClipboardFormat::Auto) else {
        panic!("Auto prefers file paths")
    };
    assert_eq!(paths, ["/tmp/a b", "/tmp/c"]);
    clear_and_finish();
}

#[test]
#[ignore = "needs a private X11 session and the built clipboard owner"]
fn the_owner_exits_when_another_client_takes_the_selection() {
    let session = private_session();
    install_helper_beside_the_image();
    set(&ClipboardContent::Text("ours".into()));
    let owner = the_record(&session.display);
    let mut taker = Command::new("setsid")
        .args(["python3", "-I"])
        .arg(script("taker", TAKER))
        .env("DISPLAY", &session.display)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("a second client takes the selection");
    let mut ready = String::new();
    std::io::BufRead::read_line(
        &mut std::io::BufReader::new(taker.stdout.take().expect("a pipe")),
        &mut ready,
    )
    .expect("the taker reports");
    assert_eq!(ready.trim(), "ready");
    wait_until("our owner ends", || gone(owner.pid));
    wait_until("our record is removed", || {
        records(&session.display).is_empty()
    });
    assert_eq!(
        foreign_read(&session.display, "UTF8_STRING").as_deref(),
        Some(b"theirs".as_slice())
    );
    let _ = taker.kill();
    let _ = taker.wait();
}
