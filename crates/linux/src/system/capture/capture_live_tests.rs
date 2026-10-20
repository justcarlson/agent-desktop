use agent_desktop_core::{ErrorCode, ScreenshotTarget};

use super::capture_live_support::*;
use crate::system::SharedBus;
use crate::system::screenshot::{screenshot, screenshot_window_frame};
use crate::tree::live_launch::Launched;

fn window_at(x: i32, y: i32) -> Geometry {
    Geometry {
        x,
        y,
        width: 320,
        height: 200,
    }
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_window_pixels_are_the_window_paint() {
    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = paint(
        &mut launched,
        "adcap-paint",
        "#ff0000",
        &window_at(200, 150),
        false,
    );
    let window = wait_for_listed(&bus, pid);
    let image = screenshot(
        &bus,
        ScreenshotTarget::ExactWindow(window),
        deadline(20_000),
    )
    .expect("the window is captured");
    let pixels = decode(&image);
    println!(
        "grab_live compositing={} size={}x{}",
        compositing(),
        pixels.width,
        pixels.height
    );
    assert!(pixels.width >= 320 && pixels.height >= 200);
    let (centre_x, centre_y) = (pixels.width / 2, pixels.height / 2);
    assert!(pixels.region_is(centre_x - 100, centre_y - 60, 200, 120, RED));
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_the_window_unit_follows_the_manager() {
    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = paint(
        &mut launched,
        "adcap-unit",
        "#ff0000",
        &window_at(200, 150),
        false,
    );
    let window = wait_for_listed(&bus, pid);
    let framed = screenshot_window_frame(&bus, &window, deadline(20_000));
    let whole = screenshot(
        &bus,
        ScreenshotTarget::ExactWindow(window),
        deadline(20_000),
    );
    println!(
        "grab_live compositing={} frame={:?} screenshot={}",
        compositing(),
        framed.as_ref().map(|image| (image.width, image.height)),
        whole.is_ok()
    );
    assert!(whole.is_ok(), "screenshot always returns the frame image");
    if compositing() {
        let error = framed.expect_err("a GTK3 window passes neither unit check under mutter");
        assert_eq!(error.code, ErrorCode::ActionFailed);
    } else {
        framed.expect("the GTK3 frame passes the unit check on openbox");
    }
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_a_full_screen_grab_equals_the_known_paint() {
    let _display = assert_private_display();
    if compositing() {
        println!("grab_live skipped: a compositing session refuses a full screen");
        return;
    }
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let area = Geometry {
        x: 100,
        y: 100,
        width: 200,
        height: 120,
    };
    paint(&mut launched, "adcap-blue", "#0000ff", &area, true);
    wait_for_x_window("adcap-blue");
    let image = screenshot(&bus, ScreenshotTarget::FullScreen, deadline(20_000))
        .expect("the screen is captured");
    let pixels = decode(&image);
    assert_eq!((pixels.width, pixels.height), (1280, 800));
    assert!(pixels.region_is(100, 100, 200, 120, BLUE));
    assert_eq!(pixels.at(10, 10), BLACK);
    assert_eq!(pixels.at(300, 100), BLACK);
    assert_eq!(pixels.at(99, 100), BLACK);
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_a_full_screen_is_refused_under_compositing() {
    let _display = assert_private_display();
    if !compositing() {
        println!("grab_live skipped: this session does not composite");
        return;
    }
    let bus = SharedBus::default();
    for target in [ScreenshotTarget::FullScreen, ScreenshotTarget::Screen(0)] {
        let error = screenshot(&bus, target, deadline(20_000)).expect_err("refused");
        assert_eq!(error.code, ErrorCode::PlatformNotSupported);
        assert_eq!(reason(&error), "compositing");
    }
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_a_covered_window_returns_its_own_paint() {
    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = paint(
        &mut launched,
        "adcap-under",
        "#ff0000",
        &window_at(200, 150),
        false,
    );
    let window = wait_for_listed(&bus, pid);
    let cover = Geometry {
        x: 100,
        y: 100,
        width: 600,
        height: 450,
    };
    paint(&mut launched, "adcap-over", "#0000ff", &cover, true);
    wait_for_x_window("adcap-over");
    if !compositing() {
        let screen = screenshot(&bus, ScreenshotTarget::FullScreen, deadline(20_000))
            .expect("the screen is captured");
        assert_eq!(
            decode(&screen).at(360, 250),
            BLUE,
            "the cover really hides the window on the screen"
        );
    }
    let image = screenshot(
        &bus,
        ScreenshotTarget::ExactWindow(window),
        deadline(20_000),
    )
    .expect("the covered window is captured");
    let pixels = decode(&image);
    println!(
        "grab_live covered size={}x{} red={} blue={}",
        pixels.width,
        pixels.height,
        pixels.count_of(RED),
        pixels.count_of(BLUE)
    );
    assert_eq!(pixels.count_of(BLUE), 0);
    let (centre_x, centre_y) = (pixels.width / 2, pixels.height / 2);
    assert!(pixels.region_is(centre_x - 100, centre_y - 60, 200, 120, RED));
    assert!(pixels.count_of(RED) >= 320 * 200);
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_a_window_partly_off_screen_reports_off_screen() {
    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = paint(
        &mut launched,
        "adcap-edge",
        "#ff0000",
        &window_at(1200, 700),
        false,
    );
    let window = wait_for_listed(&bus, pid);
    let bounds = window.bounds.expect("the window reports bounds");
    if bounds.x + bounds.width <= 1280.0 && bounds.y + bounds.height <= 800.0 {
        println!("grab_live skipped: the manager kept the window on the screen at {bounds:?}");
        return;
    }
    let outcome = screenshot(
        &bus,
        ScreenshotTarget::ExactWindow(window),
        deadline(20_000),
    );
    println!(
        "grab_live off-screen compositing={} outcome={:?}",
        compositing(),
        outcome.as_ref().map(|image| (image.width, image.height))
    );
    let error = outcome.expect_err("a window partly outside the screen is refused");
    assert_eq!(reason(&error), "off_screen");
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_the_capture_permission_follows_the_session() {
    use agent_desktop_core::Deadline;

    use crate::system::display_probe::SocketProbe;
    use crate::system::display_server::{DisplayEnv, detect};
    use crate::system::permissions::{CaptureAvailability, capture_availability};
    use crate::system::route::resolve;
    use crate::system::x11::grab::DisplayGrabber;

    let _display = assert_private_display();
    let deadline = Deadline::after(10_000).expect("a deadline builds");
    let x11 = detect(&DisplayEnv::from_process(), &SocketProbe);
    let reachable = capture_availability(resolve(x11), &DisplayGrabber, deadline);
    assert!(matches!(reachable, CaptureAvailability::Reachable));

    let runtime = std::env::var("XDG_RUNTIME_DIR").expect("a runtime directory");
    let socket = format!("{runtime}/adcap-wayland");
    let _ = std::fs::remove_file(&socket);
    let _listener = std::os::unix::net::UnixListener::bind(&socket).expect("a socket binds");
    let mut env = DisplayEnv::from_process();
    env.wayland = Some(socket);
    let wayland = detect(&env, &SocketProbe);
    let unknown = capture_availability(resolve(wayland), &DisplayGrabber, deadline);
    assert!(matches!(unknown, CaptureAvailability::Unknown));
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_a_chromium_window_is_captured_on_its_client_unit() {
    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = chrome(&mut launched);
    let window = wait_for_listed(&bus, pid);
    let framed = screenshot_window_frame(&bus, &window, deadline(20_000));
    let whole = screenshot(
        &bus,
        ScreenshotTarget::ExactWindow(window),
        deadline(20_000),
    )
    .expect("the window is captured");
    let pixels = decode(&whole);
    println!(
        "grab_live chromium compositing={} size={}x{} frame={:?}",
        compositing(),
        pixels.width,
        pixels.height,
        framed.as_ref().map(|image| (image.width, image.height))
    );
    framed.expect("Chromium's client drawable passes the unit check on both managers");
    let (centre_x, centre_y) = (pixels.width / 2, pixels.height / 2);
    assert!(pixels.region_is(centre_x - 100, centre_y - 60, 200, 120, BLUE));
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_a_qt_window_is_captured_plain_and_covered() {
    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = paint_qt(&mut launched, "adcap-qt", "#ff0000", &window_at(200, 150));
    let window = wait_for_listed(&bus, pid);
    let plain = screenshot(
        &bus,
        ScreenshotTarget::ExactWindow(window.clone()),
        deadline(20_000),
    )
    .expect("the Qt window is captured");
    let before = decode(&plain);
    let (centre_x, centre_y) = (before.width / 2, before.height / 2);
    assert!(before.region_is(centre_x - 100, centre_y - 60, 200, 120, RED));
    let cover = Geometry {
        x: 100,
        y: 100,
        width: 600,
        height: 450,
    };
    paint(&mut launched, "adcap-qt-over", "#0000ff", &cover, true);
    wait_for_x_window("adcap-qt-over");
    let covered = screenshot(
        &bus,
        ScreenshotTarget::ExactWindow(window),
        deadline(20_000),
    )
    .expect("the covered Qt window is captured");
    let after = decode(&covered);
    println!(
        "grab_live qt compositing={} plain={}x{} covered={}x{} blue={}",
        compositing(),
        before.width,
        before.height,
        after.width,
        after.height,
        after.count_of(BLUE)
    );
    assert_eq!(after.count_of(BLUE), 0);
    assert_eq!((after.width, after.height), (before.width, before.height));
    assert!(after.region_is(centre_x - 100, centre_y - 60, 200, 120, RED));
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh"]
fn grab_live_a_covered_window_that_never_repaints_is_never_the_cover() {
    use agent_desktop_core::ErrorCode;

    use crate::system::capture::png_encode::encode_png;
    use crate::system::capture::window::capture;
    use crate::system::capture::window_context::{CaptureEnv, real_decode};
    use crate::system::x11::DisplayReader;
    use crate::system::x11::grab::DisplayGrabber;

    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = paint(
        &mut launched,
        "adcap-frozen",
        "#ff0000",
        &window_at(200, 150),
        false,
    );
    let window = wait_for_listed(&bus, pid);
    let xid = x_window_id("adcap-frozen");
    let cover = Geometry {
        x: 100,
        y: 100,
        width: 600,
        height: 450,
    };
    paint(&mut launched, "adcap-frozen-over", "#0000ff", &cover, true);
    wait_for_x_window("adcap-frozen-over");
    stop(pid);
    let lookup = FixedLookup { xid };
    let env = CaptureEnv {
        grabber: &DisplayGrabber,
        stacking: &DisplayReader,
        lookup: &lookup,
        decode: real_decode,
        encode: encode_png,
    };
    let started = std::time::Instant::now();
    let outcome = capture(&env, &window, false, deadline(20_000));
    println!(
        "grab_live frozen compositing={} after={:?} outcome={:?}",
        compositing(),
        started.elapsed(),
        outcome.as_ref().map(|image| (image.width, image.height))
    );
    if let Ok(image) = &outcome {
        let pixels = decode(image);
        println!(
            "grab_live frozen red={} blue={}",
            pixels.count_of(RED),
            pixels.count_of(BLUE)
        );
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
    if compositing() {
        let image = outcome.expect("a compositor keeps the stopped window's own pixels");
        assert_eq!(decode(&image).count_of(BLUE), 0);
    } else {
        let error = outcome.expect_err("a window that cannot repaint is not captured");
        assert_eq!(error.code, ErrorCode::Timeout);
    }
}
