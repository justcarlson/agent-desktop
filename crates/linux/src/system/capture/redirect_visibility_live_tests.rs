use std::sync::Mutex;

use agent_desktop_core::Deadline;
use x11rb::protocol::xproto::ConnectionExt;

use super::super::capture_env::CaptureEnv;
use super::super::capture_live_support::*;
use super::super::window::capture;
use super::super::window_context::real_decode;
use crate::system::SharedBus;
use crate::system::x11::DisplayReader;
use crate::system::x11::drawable_info::DrawableInfo;
use crate::system::x11::grab::{DisplayGrabber, Grabber, Raw};
use crate::system::x11::grab_error::GrabError;
use crate::system::x11::grab_rect::GrabRect;
use crate::tree::live_launch::Launched;

struct UnmappingGrabber(Mutex<Option<u32>>);

impl Grabber for UnmappingGrabber {
    fn describe(&self, window: u32, deadline: Deadline) -> Result<DrawableInfo, GrabError> {
        DisplayGrabber.describe(window, deadline)
    }

    fn grab(&self, window: u32, rect: GrabRect, deadline: Deadline) -> Result<Raw, GrabError> {
        DisplayGrabber.grab(window, rect, deadline)
    }

    fn compositing(&self, deadline: Deadline) -> Result<bool, GrabError> {
        DisplayGrabber.compositing(deadline)
    }

    fn grab_redirected(
        &self,
        window: u32,
        rect: GrabRect,
        deadline: Deadline,
    ) -> Result<Raw, GrabError> {
        let (connection, _) = x11rb::connect(None).expect("private display");
        connection
            .unmap_window(window)
            .expect("unmap")
            .check()
            .expect("unmapped");
        *self.0.lock().expect("hidden drawable") = Some(window);
        DisplayGrabber.grab_redirected(window, rect, deadline)
    }
}

#[test]
#[ignore = "unmaps an owned drawable between survey and redirected capture on private X11"]
fn a_redirect_that_loses_visibility_keeps_the_real_x11_error() {
    let _display = assert_private_display();
    let bus = SharedBus::default();
    let mut launched = Launched::new();
    let geometry = Geometry {
        x: 200,
        y: 150,
        width: 320,
        height: 200,
    };
    let pid = paint(&mut launched, "redirect-visibility", "#ff0000", &geometry);
    let mut window = wait_for_listed(&bus, pid);
    let lookup = FixedLookup {
        xid: x_window_id("redirect-visibility"),
    };
    window.bounds = Some(
        DisplayGrabber
            .describe(lookup.xid, deadline(5_000))
            .expect("client geometry")
            .rect
            .into(),
    );
    let grabber = UnmappingGrabber(Mutex::new(None));
    let env = CaptureEnv {
        grabber: &grabber,
        stacking: &DisplayReader,
        lookup: &lookup,
        decode: real_decode,
        encode: super::super::png_encode::encode_png,
        stopped: crate::system::proc_stat::is_stopped,
    };
    let image = capture(&env, &window, true, deadline(20_000)).expect("visible window captures");
    let path = format!(
        "{}/redirect-visible.png",
        std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("private session")
    );
    std::fs::write(&path, &image.data).expect("capture file");
    let pixels = std::process::Command::new("python3").args(["-c", r#"
import gi, sys
gi.require_version('GdkPixbuf', '2.0')
from gi.repository import GdkPixbuf
image = GdkPixbuf.Pixbuf.new_from_file(sys.argv[1])
pixels = bytes(image.get_pixels())
assert sum(pixels[i:i+3] == b'\xff\x00\x00' for i in range(0, len(pixels)-2, image.get_n_channels())) > 10000
"#, &path]).status().expect("independent pixel oracle");
    assert!(pixels.success());
    let cover = paint(&mut launched, "redirect-cover", "#0000ff", &geometry);
    wait_for_listed(&bus, cover);
    let error =
        capture(&env, &window, true, deadline(20_000)).expect_err("unmapped during redirect");
    assert!(error.message.contains("not visible"), "{error:?}");
    assert!(
        !error
            .suggestion
            .unwrap_or_default()
            .contains("focus-window")
    );
    let drawable = grabber
        .0
        .lock()
        .expect("hidden drawable")
        .expect("redirect route was reached");
    let oracle = std::process::Command::new("xwininfo")
        .args(["-id", &drawable.to_string()])
        .output()
        .expect("independent map-state oracle");
    assert!(String::from_utf8_lossy(&oracle.stdout).contains("Map State: IsUnMapped"));
}
