use agent_desktop_core::ScreenshotTarget;

use super::capture_live_support::*;
use crate::system::SharedBus;
use crate::system::screenshot::screenshot;
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
fn grab_live_a_window_capture_leaves_out_the_client_side_shadow() {
    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let pid = paint_decorated(
        &mut launched,
        "adcap-shadow",
        "#ff0000",
        &window_at(200, 150),
    );
    let window = wait_for_listed(&bus, pid);
    wait_for_x_window("adcap-shadow");
    let xid = x_window_id("adcap-shadow");
    let shadow = shadow_of(xid);
    let (client_width, client_height) = client_size(xid);
    let image = screenshot(
        &bus,
        ScreenshotTarget::ExactWindow(window),
        deadline(20_000),
    )
    .expect("the window is captured");
    let pixels = decode(&image);
    println!(
        "grab_live shadow compositing={} extents={shadow:?} client={client_width}x{client_height} image={}x{}",
        compositing(),
        pixels.width,
        pixels.height
    );
    let [left, right, top, bottom] = shadow.unwrap_or([0; 4]);
    if let Some(extents) = shadow {
        assert!(extents.iter().any(|edge| *edge > 0), "a shadow is drawn");
        assert_eq!(
            (pixels.width as i32, pixels.height as i32),
            (client_width - left - right, client_height - top - bottom)
        );
    }
    let (width, height) = (pixels.width, pixels.height);
    for (x, y) in [
        (width / 2, 0),
        (width / 2, height - 1),
        (0, height / 2),
        (width - 1, height / 2),
    ] {
        assert_ne!(pixels.at(x, y), BLACK, "no black margin at ({x}, {y})");
    }
}
