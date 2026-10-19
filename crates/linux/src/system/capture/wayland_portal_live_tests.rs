use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ErrorCode, ScreenshotTarget};

use super::wayland_portal::capture_wayland;
use super::wayland_portal_live_support::{
    BLUE, DIALOG_SHOWN, KEY_RETURN, KEY_TAB, Keys, World, boot, stored_screenshot_grants,
};

fn home_screenshots() -> BTreeSet<String> {
    let home = PathBuf::from(std::env::var("HOME").expect("a home directory"));
    std::fs::read_dir(home)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.starts_with("Screenshot") && name.ends_with(".png"))
                .collect()
        })
        .unwrap_or_default()
}

fn capture_while(
    world: &World,
    ms: u64,
    answer: impl FnOnce(&Keys),
) -> Result<agent_desktop_core::ImageBuffer, agent_desktop_core::AdapterError> {
    let deadline = Deadline::after(ms).expect("a deadline");
    let worker =
        std::thread::spawn(move || capture_wayland(&ScreenshotTarget::FullScreen, deadline));
    std::thread::sleep(DIALOG_SHOWN);
    answer(&world.keys);
    worker.join().expect("the capture thread ends")
}

fn blue_pixels(png_bytes: &[u8]) -> (u32, u32, usize) {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().expect("a PNG header");
    let mut buffer = vec![0; reader.output_buffer_size().expect("a size")];
    let info = reader.next_frame(&mut buffer).expect("a frame");
    let channels = info.color_type.samples();
    let count = buffer[..info.buffer_size()]
        .chunks_exact(channels)
        .filter(|pixel| pixel[..3] == BLUE)
        .count();
    (info.width, info.height, count)
}

#[test]
#[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
fn no_answer_is_a_timeout_that_names_the_dialog_and_leaves_no_file() {
    let world = boot();
    let before = home_screenshots();
    let started = Instant::now();
    let silent = capture_while(&world, 9_000, |_| {}).expect_err("no answer is a timeout");
    println!(
        "portal_live silent {:?} after {:?}",
        silent.code,
        started.elapsed()
    );
    assert_eq!(silent.code, ErrorCode::Timeout);
    assert!(
        silent
            .suggestion
            .as_deref()
            .is_some_and(|text| text.contains("dialog"))
    );
    assert!(started.elapsed() < Duration::from_secs(20));
    assert_eq!(home_screenshots(), before);
}

#[test]
#[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
fn allow_gives_the_desktop_and_leaves_no_file() {
    let world = boot();
    let before = home_screenshots();
    let allowed = capture_while(&world, 25_000, |keys| {
        keys.press(KEY_TAB);
        std::thread::sleep(Duration::from_millis(400));
        keys.press(KEY_RETURN);
    })
    .expect("the desktop is captured after Allow");
    let (width, height, blue) = blue_pixels(&allowed.data);
    println!("portal_live allowed {width}x{height} blue={blue}");
    assert_eq!((allowed.width, allowed.height), (1280, 800));
    assert_eq!((width, height), (1280, 800));
    assert!(
        blue >= 100_000,
        "the painter window is in the image: {blue}"
    );
    assert_eq!(home_screenshots(), before, "the portal's file is removed");
}

#[test]
#[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
fn deny_is_permission_denied_and_leaves_no_file() {
    let world = boot();
    let before = home_screenshots();
    let denied = capture_while(&world, 25_000, |keys| keys.press(KEY_RETURN))
        .map(|image| image.data.len())
        .expect_err("Deny is a refusal");
    println!("portal_live denied {:?}", denied.code);
    assert_eq!(denied.code, ErrorCode::PermDenied);
    assert_eq!(home_screenshots(), before);
}

fn quick_capture() -> (
    Result<agent_desktop_core::ImageBuffer, agent_desktop_core::AdapterError>,
    Duration,
) {
    let started = Instant::now();
    let deadline = Deadline::after(15_000).expect("a deadline");
    let outcome = capture_wayland(&ScreenshotTarget::FullScreen, deadline);
    (outcome, started.elapsed())
}

#[test]
#[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
fn one_allow_is_remembered_for_the_host_process_and_later_calls_show_no_dialog() {
    let world = boot();
    assert_eq!(stored_screenshot_grants(), "");
    capture_while(&world, 25_000, |keys| {
        keys.press(KEY_TAB);
        std::thread::sleep(Duration::from_millis(400));
        keys.press(KEY_RETURN);
    })
    .expect("the first call is allowed by a person");
    let stored = stored_screenshot_grants();
    println!("portal_live grants after allow: {stored}");
    assert_eq!(stored, r#"screenshot=Some({"": ["yes"]})"#);
    for call in [2, 3] {
        let (outcome, took) = quick_capture();
        println!("portal_live call{call} {:?} in {took:?}", outcome.is_ok());
        assert!(outcome.is_ok(), "call {call} needs no dialog");
        assert!(took < Duration::from_secs(5), "{took:?}");
    }
}

#[test]
#[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
fn one_deny_is_remembered_and_later_calls_are_refused_at_once() {
    let world = boot();
    capture_while(&world, 25_000, |keys| keys.press(KEY_RETURN)).expect_err("denied");
    let stored = stored_screenshot_grants();
    println!("portal_live grants after deny: {stored}");
    assert_eq!(stored, r#"screenshot=Some({"": ["no"]})"#);
    let (outcome, took) = quick_capture();
    println!(
        "portal_live second call {:?} in {took:?}",
        outcome.as_ref().map_err(|e| e.code.clone()).err()
    );
    assert_eq!(outcome.expect_err("refused").code, ErrorCode::PermDenied);
    assert!(took < Duration::from_secs(5), "{took:?}");
}
