use std::process::Command;
use std::time::{Duration, Instant};

use agent_desktop_core::Deadline;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, CreateWindowAux, WindowClass};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

use super::*;
use crate::system::capture::pixels::to_rgb;
use crate::system::capture::png_encode::encode_png;
use crate::system::x11::map_state::MapState;

const FILL: [u8; 3] = [0x33, 0x66, 0x99];
const ORIGIN: (i16, i16) = (100, 90);
const SIZE: (u16, u16) = (200, 120);

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("a deadline builds")
}

fn private_display() -> String {
    let private = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let display = std::env::var("DISPLAY").expect("DISPLAY");
    assert!(!private.is_empty());
    assert_ne!(display, ":0", "the live display must never be captured");
    display
}

struct Flat {
    connection: RustConnection,
    window: u32,
}

impl Flat {
    fn new(display: &str) -> Self {
        let (connection, screen) = RustConnection::connect(Some(display)).expect("connect");
        let root = connection.setup().roots[screen].root;
        let window = connection.generate_id().expect("an id");
        let pixel = u32::from_be_bytes([0, FILL[0], FILL[1], FILL[2]]);
        connection
            .create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                window,
                root,
                ORIGIN.0,
                ORIGIN.1,
                SIZE.0,
                SIZE.1,
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &CreateWindowAux::new()
                    .background_pixel(pixel)
                    .override_redirect(1),
            )
            .expect("create");
        Self { connection, window }
    }

    fn map(&self) {
        self.connection.map_window(self.window).expect("map");
        self.connection.sync().expect("sync");
    }

    fn unmap(&self) {
        self.connection.unmap_window(self.window).expect("unmap");
        self.connection.sync().expect("sync");
    }
}

fn xwd_pixel(window: u32, x: usize, y: usize, dir: &str) -> [u8; 3] {
    let path = format!("{dir}/flat.xwd");
    let status = Command::new("xwd")
        .args(["-silent", "-id", &window.to_string(), "-out", &path])
        .status()
        .expect("xwd runs");
    assert!(status.success());
    let file = std::fs::read(&path).expect("xwd output");
    let field = |index: usize| {
        u32::from_be_bytes(
            file[index * 4..index * 4 + 4]
                .try_into()
                .expect("four bytes"),
        )
    };
    let (header, colours) = (field(0) as usize, field(19) as usize);
    let (big_endian, bits, line) = (field(7) == 1, field(11) as usize, field(12) as usize);
    let bytes = bits / 8;
    let start = header + colours * 12 + y * line + x * bytes;
    let pixel = &file[start..start + bytes];
    let value = if big_endian {
        pixel.iter().fold(0u32, |v, b| (v << 8) | u32::from(*b))
    } else {
        pixel
            .iter()
            .rev()
            .fold(0u32, |v, b| (v << 8) | u32::from(*b))
    };
    let channel = |mask: u32| ((value & mask) >> mask.trailing_zeros()) as u8;
    [channel(field(14)), channel(field(15)), channel(field(16))]
}

fn grab_when_painted(window: u32, rect: GrabRect) -> Raw {
    let started = Instant::now();
    loop {
        match DisplayGrabber.grab(window, rect, deadline()) {
            Ok(raw) => return raw,
            Err(error) if started.elapsed() > Duration::from_secs(10) => {
                panic!("the window never became grabbable: {error:?}")
            }
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]
fn grab_live_a_flat_window_reads_back_its_fill_and_matches_an_independent_capture() {
    let display = private_display();
    let dir = std::env::var("AGENT_DESKTOP_PRIVATE_SESSION").expect("a private session");
    let flat = Flat::new(&display);
    flat.map();
    let whole = GrabRect {
        x: 0,
        y: 0,
        width: SIZE.0,
        height: SIZE.1,
    };
    let raw = grab_when_painted(flat.window, whole);
    println!("grab_live format={:?} stride={}", raw.format, raw.stride);
    assert_eq!(
        (raw.width, raw.height),
        (u32::from(SIZE.0), u32::from(SIZE.1))
    );
    let rgb = to_rgb(&raw.format, &raw.data, raw.width, raw.height, raw.stride).expect("converts");
    assert!(rgb.chunks_exact(3).all(|pixel| pixel == FILL));
    assert_eq!(xwd_pixel(flat.window, 50, 40, &dir), FILL);
    let png = encode_png(&rgb, raw.width, raw.height, 1.0).expect("encodes");
    assert_eq!(
        agent_desktop_core::parse_png_dimensions(&png.data),
        Some((200, 120))
    );
    let part = DisplayGrabber
        .grab(
            flat.window,
            GrabRect {
                x: 10,
                y: 20,
                width: 50,
                height: 30,
            },
            deadline(),
        )
        .expect("a crop grabs");
    assert_eq!((part.width, part.height), (50, 30));
    let crop = to_rgb(&part.format, &part.data, 50, 30, part.stride).expect("converts");
    assert!(crop.chunks_exact(3).all(|pixel| pixel == FILL));
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]
fn grab_live_describes_the_window_and_names_each_refusal() {
    let display = private_display();
    let flat = Flat::new(&display);
    flat.map();
    let info = DisplayGrabber
        .describe(flat.window, deadline())
        .expect("describes");
    println!("grab_live info={info:?}");
    assert_eq!(
        (info.rect.x, info.rect.y, info.rect.width, info.rect.height),
        (100, 90, 200, 120)
    );
    assert_eq!(info.map_state, MapState::Viewable);
    assert!(info.override_redirect);
    assert_eq!(info.parent, None);
    let root = DisplayGrabber.describe(0, deadline()).expect("the root");
    assert_eq!((root.rect.width, root.rect.height), (1280, 800));
    let beyond = GrabRect {
        x: 150,
        y: 0,
        width: 100,
        height: 50,
    };
    assert_eq!(
        DisplayGrabber.grab(flat.window, beyond, deadline()).err(),
        Some(GrabError::OffScreen)
    );
    flat.unmap();
    let whole = GrabRect {
        x: 0,
        y: 0,
        width: 10,
        height: 10,
    };
    assert_eq!(
        DisplayGrabber.grab(flat.window, whole, deadline()).err(),
        Some(GrabError::Unviewable)
    );
    let gone = flat.window;
    flat.connection.destroy_window(gone).expect("destroy");
    flat.connection.sync().expect("sync");
    assert_eq!(
        DisplayGrabber.grab(gone, whole, deadline()).err(),
        Some(GrabError::Gone)
    );
    assert_eq!(
        DisplayGrabber.describe(gone, deadline()).err(),
        Some(GrabError::Gone)
    );
}

#[test]
#[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]
fn grab_live_the_root_grab_and_the_compositing_read_answer_on_the_private_screen() {
    private_display();
    assert_eq!(DisplayGrabber.compositing(deadline()), Ok(false));
    let rect = GrabRect {
        x: 0,
        y: 0,
        width: 1280,
        height: 800,
    };
    let raw = DisplayGrabber
        .grab(0, rect, deadline())
        .expect("the root grabs");
    assert_eq!((raw.width, raw.height), (1280, 800));
    assert_eq!(raw.data.len(), raw.stride * 800);
}
