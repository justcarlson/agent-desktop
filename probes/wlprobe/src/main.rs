mod hyprmap {
    #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables, non_upper_case_globals, non_snake_case, unused_imports, missing_docs, clippy::all)]
    use wayland_client;
    use wayland_client::protocol::*;
    use wayland_protocols::ext::foreign_toplevel_list::v1::client::*;
    use wayland_protocols_wlr::foreign_toplevel::v1::client::*;
    pub mod __interfaces {
        use wayland_client::protocol::__interfaces::*;
        use wayland_protocols::ext::foreign_toplevel_list::v1::client::__interfaces::*;
        use wayland_protocols_wlr::foreign_toplevel::v1::client::__interfaces::*;
        wayland_scanner::generate_interfaces!("protocols/hyprland-toplevel-mapping-v1.xml");
    }
    use self::__interfaces::*;
    wayland_scanner::generate_client_code!("protocols/hyprland-toplevel-mapping-v1.xml");
}

use hyprmap::hyprland_toplevel_mapping_manager_v1::HyprlandToplevelMappingManagerV1;
use hyprmap::hyprland_toplevel_window_mapping_handle_v1::{self as maph, HyprlandToplevelWindowMappingHandleV1};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::{AsFd, AsRawFd, FromRawFd};
use std::time::{Duration, Instant};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_buffer, wl_compositor, wl_output, wl_region, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface};
use wayland_client::{event_created_child, Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::{self as exth, ExtForeignToplevelHandleV1};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::{self as extl, ExtForeignToplevelListV1};
use wayland_protocols::ext::image_capture_source::v1::client::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1;
use wayland_protocols::ext::image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::{self as icf, ExtImageCopyCaptureFrameV1};
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::{self as icm, ExtImageCopyCaptureManagerV1};
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::{self as ics, ExtImageCopyCaptureSessionV1};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{self as wlrh, ZwlrForeignToplevelHandleV1};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{self as wlrm, ZwlrForeignToplevelManagerV1};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{self as lsh, ZwlrLayerShellV1};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{self as lsf, ZwlrLayerSurfaceV1};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{self as scf, ZwlrScreencopyFrameV1};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

struct ExtTop {
    h: ExtForeignToplevelHandleV1,
    identifier: String,
    title: String,
    app_id: String,
    addr: Option<u64>,
    map_failed: bool,
}
struct WlrTop {
    h: ZwlrForeignToplevelHandleV1,
    title: String,
    app_id: String,
    states: Vec<u32>,
    addr: Option<u64>,
    events: Vec<String>,
}
#[derive(Default)]
struct Sess {
    w: u32,
    h: u32,
    formats: Vec<u32>,
    done: bool,
    stopped: bool,
}
#[derive(Default)]
struct Frame {
    ready: bool,
    failed: Option<String>,
    damage: Vec<(i32, i32, i32, i32)>,
}
#[derive(Default)]
struct Sc {
    format: u32,
    w: u32,
    h: u32,
    stride: u32,
    buffer_done: bool,
    ready: bool,
    failed: bool,
    flags: u32,
}
#[derive(Default)]
struct State {
    ext: Vec<ExtTop>,
    wlr: Vec<WlrTop>,
    sess: Sess,
    frame: Frame,
    sc: Sc,
    configures: Vec<(usize, u32, u32, u32)>,
    layer_closed: bool,
}

macro_rules! noop {
    ($($t:ty),*) => {$(
        impl Dispatch<$t, ()> for State {
            fn event(_: &mut Self, _: &$t, _: <$t as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
        }
    )*};
}
noop!(wl_compositor::WlCompositor, wl_shm::WlShm, wl_shm_pool::WlShmPool, wl_buffer::WlBuffer, wl_output::WlOutput, wl_region::WlRegion, wl_surface::WlSurface, wl_seat::WlSeat,
    HyprlandToplevelMappingManagerV1, ExtForeignToplevelImageCaptureSourceManagerV1, ExtImageCaptureSourceV1, ExtImageCopyCaptureManagerV1,
    ZwpVirtualKeyboardManagerV1, ZwpVirtualKeyboardV1, ZwlrLayerShellV1, ZwlrScreencopyManagerV1);

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(_: &mut Self, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<ExtForeignToplevelListV1, ()> for State {
    fn event(st: &mut Self, _: &ExtForeignToplevelListV1, ev: extl::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let extl::Event::Toplevel { toplevel } = ev {
            st.ext.push(ExtTop { h: toplevel, identifier: String::new(), title: String::new(), app_id: String::new(), addr: None, map_failed: false });
        }
    }
    event_created_child!(State, ExtForeignToplevelListV1, [extl::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ())]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for State {
    fn event(st: &mut Self, p: &ExtForeignToplevelHandleV1, ev: exth::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        let Some(t) = st.ext.iter_mut().find(|t| t.h == *p) else { return };
        match ev {
            exth::Event::Identifier { identifier } => t.identifier = identifier,
            exth::Event::Title { title } => t.title = title,
            exth::Event::AppId { app_id } => t.app_id = app_id,
            _ => {}
        }
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for State {
    fn event(st: &mut Self, _: &ZwlrForeignToplevelManagerV1, ev: wlrm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wlrm::Event::Toplevel { toplevel } = ev {
            st.wlr.push(WlrTop { h: toplevel, title: String::new(), app_id: String::new(), states: vec![], addr: None, events: vec![] });
        }
    }
    event_created_child!(State, ZwlrForeignToplevelManagerV1, [wlrm::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ())]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for State {
    fn event(st: &mut Self, p: &ZwlrForeignToplevelHandleV1, ev: wlrh::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        let Some(t) = st.wlr.iter_mut().find(|t| t.h == *p) else { return };
        match ev {
            wlrh::Event::Title { title } => t.title = title,
            wlrh::Event::AppId { app_id } => t.app_id = app_id,
            wlrh::Event::State { state } => {
                t.states = state.chunks_exact(4).map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]])).collect();
                t.events.push(format!("state{:?}", t.states));
            }
            wlrh::Event::Closed => t.events.push("closed".into()),
            _ => {}
        }
    }
}

impl Dispatch<HyprlandToplevelWindowMappingHandleV1, (bool, usize)> for State {
    fn event(st: &mut Self, _: &HyprlandToplevelWindowMappingHandleV1, ev: maph::Event, d: &(bool, usize), _: &Connection, _: &QueueHandle<Self>) {
        let (is_wlr, i) = *d;
        match ev {
            maph::Event::WindowAddress { address_hi, address } => {
                let a = ((address_hi as u64) << 32) | address as u64;
                if is_wlr { st.wlr[i].addr = Some(a) } else { st.ext[i].addr = Some(a) }
            }
            maph::Event::Failed => {
                if !is_wlr { st.ext[i].map_failed = true }
            }
        }
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for State {
    fn event(st: &mut Self, _: &ExtImageCopyCaptureSessionV1, ev: ics::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match ev {
            ics::Event::BufferSize { width, height } => { st.sess.w = width; st.sess.h = height }
            ics::Event::ShmFormat { format: WEnum::Value(f) } => st.sess.formats.push(f.into()),
            ics::Event::Done => st.sess.done = true,
            ics::Event::Stopped => st.sess.stopped = true,
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for State {
    fn event(st: &mut Self, _: &ExtImageCopyCaptureFrameV1, ev: icf::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match ev {
            icf::Event::Damage { x, y, width, height } => st.frame.damage.push((x, y, width, height)),
            icf::Event::Ready => st.frame.ready = true,
            icf::Event::Failed { reason } => st.frame.failed = Some(format!("{reason:?}")),
            _ => {}
        }
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for State {
    fn event(st: &mut Self, _: &ZwlrScreencopyFrameV1, ev: scf::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        match ev {
            scf::Event::Buffer { format: WEnum::Value(f), width, height, stride } => {
                let f: u32 = f.into();
                if st.sc.w == 0 || f == 1 || f == 0 { st.sc.format = f; st.sc.w = width; st.sc.h = height; st.sc.stride = stride }
            }
            scf::Event::BufferDone => st.sc.buffer_done = true,
            scf::Event::Flags { flags: WEnum::Value(f) } => st.sc.flags = f.bits(),
            scf::Event::Ready { .. } => st.sc.ready = true,
            scf::Event::Failed => st.sc.failed = true,
            _ => {}
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, usize> for State {
    fn event(st: &mut Self, _: &ZwlrLayerSurfaceV1, ev: lsf::Event, i: &usize, _: &Connection, _: &QueueHandle<Self>) {
        match ev {
            lsf::Event::Configure { serial, width, height } => st.configures.push((*i, serial, width, height)),
            lsf::Event::Closed => st.layer_closed = true,
            _ => {}
        }
    }
}

fn pump(q: &mut EventQueue<State>, st: &mut State, ms: i32) {
    q.dispatch_pending(st).unwrap();
    q.flush().unwrap();
    if let Some(g) = q.prepare_read() {
        let mut p = libc::pollfd { fd: g.connection_fd().as_raw_fd(), events: libc::POLLIN, revents: 0 };
        if unsafe { libc::poll(&mut p, 1, ms) } > 0 {
            let _ = g.read();
        }
    }
    q.dispatch_pending(st).unwrap();
}

fn wait(q: &mut EventQueue<State>, st: &mut State, ms: u64, pred: impl Fn(&State) -> bool) -> bool {
    let end = Instant::now() + Duration::from_millis(ms);
    while !pred(st) {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() { return false }
        pump(q, st, left.as_millis().max(1) as i32);
    }
    true
}

fn memfd(size: usize) -> File {
    let fd = unsafe { libc::memfd_create(c"wlprobe".as_ptr(), libc::MFD_CLOEXEC) };
    assert!(fd >= 0);
    let f = unsafe { File::from_raw_fd(fd) };
    f.set_len(size as u64).unwrap();
    f
}

fn shm_buffer(shm: &wl_shm::WlShm, qh: &QueueHandle<State>, w: u32, h: u32, stride: u32, fmt: u32) -> (File, wl_buffer::WlBuffer) {
    let size = (stride * h) as usize;
    let f = memfd(size);
    let pool = shm.create_pool(f.as_fd(), size as i32, qh, ());
    let b = pool.create_buffer(0, w as i32, h as i32, stride as i32, wl_shm::Format::try_from(fmt).unwrap(), qh, ());
    pool.destroy();
    (f, b)
}

fn read_all(f: &mut File) -> Vec<u8> {
    let mut v = vec![];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_to_end(&mut v).unwrap();
    v
}

fn write_png(path: &str, w: u32, h: u32, stride: u32, px: &[u8], yinvert: bool) {
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        let row = if yinvert { h - 1 - y } else { y };
        for x in 0..w {
            let o = (row * stride + x * 4) as usize;
            rgb.extend_from_slice(&[px[o + 2], px[o + 1], px[o]]);
        }
    }
    let mut e = png::Encoder::new(File::create(path).unwrap(), w, h);
    e.set_color(png::ColorType::Rgb);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(&rgb).unwrap();
}

fn hex(a: Option<u64>) -> String {
    a.map(|a| format!("0x{a:x}")).unwrap_or_else(|| "null".into())
}

struct Ctx {
    q: EventQueue<State>,
    qh: QueueHandle<State>,
    st: State,
    g: wayland_client::globals::GlobalList,
}

fn connect() -> Ctx {
    let conn = Connection::connect_to_env().expect("wayland connect");
    let (g, q) = registry_queue_init::<State>(&conn).unwrap();
    let qh = q.handle();
    Ctx { q, qh, st: State::default(), g }
}

fn load_toplevels(c: &mut Ctx) {
    let _el: ExtForeignToplevelListV1 = c.g.bind(&c.qh, 1..=1, ()).expect("ext_foreign_toplevel_list_v1");
    let _wm: ZwlrForeignToplevelManagerV1 = c.g.bind(&c.qh, 1..=3, ()).expect("zwlr_foreign_toplevel_manager_v1");
    let mm: HyprlandToplevelMappingManagerV1 = c.g.bind(&c.qh, 1..=1, ()).expect("hyprland_toplevel_mapping_manager_v1");
    for _ in 0..3 { c.q.roundtrip(&mut c.st).unwrap(); }
    for i in 0..c.st.ext.len() { mm.get_window_for_toplevel(&c.st.ext[i].h, &c.qh, (false, i)); }
    for i in 0..c.st.wlr.len() { mm.get_window_for_toplevel_wlr(&c.st.wlr[i].h, &c.qh, (true, i)); }
    for _ in 0..2 { c.q.roundtrip(&mut c.st).unwrap(); }
}

fn cmd_list() {
    let mut c = connect();
    load_toplevels(&mut c);
    for t in &c.st.ext {
        let w = c.st.wlr.iter().find(|w| w.addr.is_some() && w.addr == t.addr);
        println!("{{\"ext_identifier\":{:?},\"title\":{:?},\"app_id\":{:?},\"address\":{:?},\"map_failed\":{},\"wlr_states\":{:?}}}",
            t.identifier, t.title, t.app_id, hex(t.addr), t.map_failed, w.map(|w| w.states.clone()).unwrap_or_default());
    }
    eprintln!("ext={} wlr={}", c.st.ext.len(), c.st.wlr.len());
}

fn cmd_activate(addr: &str) {
    let mut c = connect();
    let seat: wl_seat::WlSeat = c.g.bind(&c.qh, 1..=7, ()).unwrap();
    load_toplevels(&mut c);
    let target = u64::from_str_radix(addr.trim_start_matches("0x"), 16).unwrap();
    let i = c.st.wlr.iter().position(|w| w.addr == Some(target)).expect("no wlr toplevel for address");
    for w in c.st.wlr.iter_mut() { w.events.clear() }
    println!("before: {:?}", c.st.wlr.iter().map(|w| (hex(w.addr), w.title.clone(), w.states.clone())).collect::<Vec<_>>());
    c.st.wlr[i].h.activate(&seat);
    let t0 = Instant::now();
    let st_ref = &mut c.st;
    wait(&mut c.q, st_ref, 1500, |s| s.wlr[i].states.contains(&2));
    let dt = t0.elapsed().as_millis();
    pump(&mut c.q, &mut c.st, 300);
    println!("after_ms={dt} after: {:?}", c.st.wlr.iter().map(|w| (hex(w.addr), w.title.clone(), w.states.clone(), w.events.clone())).collect::<Vec<_>>());
}

fn keymap(chars: &[char]) -> String {
    let mut codes = String::new();
    let mut syms = String::new();
    for (i, ch) in chars.iter().enumerate() {
        let cp = *ch as u32;
        let sym = if cp == 9 { 0xff09 } else if cp == 10 { 0xff0d } else if (0x20..=0x7e).contains(&cp) || (0xa0..=0xff).contains(&cp) { cp } else { 0x0100_0000 | cp };
        codes.push_str(&format!("<K{i}> = {};\n", i + 9));
        syms.push_str(&format!("key <K{i}> {{[ 0x{sym:x} ]}};\n"));
    }
    format!("xkb_keymap {{\nxkb_keycodes \"wlprobe\" {{ minimum = 8; maximum = {}; {codes} }};\nxkb_types \"wlprobe\" {{ include \"complete\" }};\nxkb_compatibility \"wlprobe\" {{ include \"complete\" }};\nxkb_symbols \"wlprobe\" {{ {syms} }};\n}};\n", chars.len() + 9)
}

fn cmd_type(text: &str, delay: u64) {
    let mut c = connect();
    let seat: wl_seat::WlSeat = c.g.bind(&c.qh, 1..=7, ()).unwrap();
    let vm: ZwpVirtualKeyboardManagerV1 = c.g.bind(&c.qh, 1..=1, ()).expect("zwp_virtual_keyboard_manager_v1");
    let vk = vm.create_virtual_keyboard(&seat, &c.qh, ());
    let mut uniq: Vec<char> = vec![];
    for ch in text.chars() { if !uniq.contains(&ch) { uniq.push(ch) } }
    let km = keymap(&uniq);
    let mut f = memfd(km.len() + 1);
    f.write_all(km.as_bytes()).unwrap();
    f.write_all(&[0]).unwrap();
    vk.keymap(1, f.as_fd(), (km.len() + 1) as u32);
    c.q.roundtrip(&mut c.st).unwrap();
    let t0 = Instant::now();
    let mut sent = 0;
    for ch in text.chars() {
        let k = uniq.iter().position(|u| *u == ch).unwrap() as u32 + 1;
        let t = t0.elapsed().as_millis() as u32;
        vk.key(t, k, 1);
        vk.key(t + 1, k, 0);
        sent += 2;
        c.q.roundtrip(&mut c.st).unwrap();
        std::thread::sleep(Duration::from_millis(delay));
    }
    let hold: u64 = std::env::var("WLPROBE_HOLD_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    std::thread::sleep(Duration::from_millis(hold));
    vk.destroy();
    c.q.roundtrip(&mut c.st).unwrap();
    println!("sent_events={sent} keymap_bytes={} hold_ms={hold}", km.len());
}

fn cmd_rawkeys(keymap_file: &str, codes: &[u32]) {
    let mut c = connect();
    let seat: wl_seat::WlSeat = c.g.bind(&c.qh, 1..=7, ()).unwrap();
    let vm: ZwpVirtualKeyboardManagerV1 = c.g.bind(&c.qh, 1..=1, ()).expect("zwp_virtual_keyboard_manager_v1");
    let vk = vm.create_virtual_keyboard(&seat, &c.qh, ());
    let km = std::fs::read(keymap_file).unwrap();
    let mut f = memfd(km.len() + 1);
    f.write_all(&km).unwrap();
    f.write_all(&[0]).unwrap();
    vk.keymap(1, f.as_fd(), (km.len() + 1) as u32);
    vk.modifiers(0, 0, 0, 0);
    c.q.roundtrip(&mut c.st).unwrap();
    for (n, k) in codes.iter().enumerate() {
        vk.key(n as u32 * 10, *k, 1);
        vk.key(n as u32 * 10 + 1, *k, 0);
        c.q.roundtrip(&mut c.st).unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(std::env::var("WLPROBE_HOLD_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(0)));
    vk.destroy();
    c.q.roundtrip(&mut c.st).unwrap();
    println!("rawkeys sent={}", codes.len() * 2);
}

fn cmd_capture(addr: &str, out: &str, between: Option<&str>, timeout: u64) {
    let mut c = connect();
    let shm: wl_shm::WlShm = c.g.bind(&c.qh, 1..=1, ()).unwrap();
    let srcm: ExtForeignToplevelImageCaptureSourceManagerV1 = c.g.bind(&c.qh, 1..=1, ()).expect("ext_foreign_toplevel_image_capture_source_manager_v1");
    let cm: ExtImageCopyCaptureManagerV1 = c.g.bind(&c.qh, 1..=1, ()).expect("ext_image_copy_capture_manager_v1");
    load_toplevels(&mut c);
    let target = u64::from_str_radix(addr.trim_start_matches("0x"), 16).unwrap();
    let i = c.st.ext.iter().position(|t| t.addr == Some(target)).expect("no ext toplevel for address");
    println!("target ext_identifier={} title={:?}", c.st.ext[i].identifier, c.st.ext[i].title);
    let src = srcm.create_source(&c.st.ext[i].h, &c.qh, ());
    let sess = cm.create_session(&src, icm::Options::empty(), &c.qh, ());
    let ok = wait(&mut c.q, &mut c.st, 2000, |s| s.sess.done || s.sess.stopped);
    println!("session done={ok} stopped={} size={}x{} shm_formats={:?}", c.st.sess.stopped, c.st.sess.w, c.st.sess.h, c.st.sess.formats);
    if !ok || c.st.sess.stopped { return }
    let fmt = if c.st.sess.formats.contains(&1) { 1 } else if c.st.sess.formats.contains(&0) { 0 } else { c.st.sess.formats[0] };
    let (w, h) = (c.st.sess.w, c.st.sess.h);
    let (mut file, buf) = shm_buffer(&shm, &c.qh, w, h, w * 4, fmt);
    let mut shots: Vec<Option<Vec<u8>>> = vec![];
    let labels = ["f1_first", "f2_static", "f3_after"];
    for (n, label) in labels.iter().enumerate() {
        if n == 2 {
            if let Some(cmd) = between {
                let st = std::process::Command::new("sh").arg("-c").arg(cmd).status().unwrap();
                println!("between: {cmd} -> {st}");
            }
        }
        c.st.frame = Frame::default();
        let fr = sess.create_frame(&c.qh, ());
        fr.attach_buffer(&buf);
        fr.damage_buffer(0, 0, w as i32, h as i32);
        fr.capture();
        let t0 = Instant::now();
        let got = wait(&mut c.q, &mut c.st, timeout, |s| s.frame.ready || s.frame.failed.is_some());
        let ms = t0.elapsed().as_millis();
        println!("{label}: arrived={got} ready={} failed={:?} ms={ms} damage={:?}", c.st.frame.ready, c.st.frame.failed, c.st.frame.damage);
        fr.destroy();
        if c.st.frame.ready {
            let px = read_all(&mut file);
            write_png(&format!("{out}-{label}.png"), w, h, w * 4, &px, false);
            shots.push(Some(px));
        } else {
            shots.push(None);
            if c.st.frame.failed.is_none() {
                c.st.frame = Frame::default();
            }
        }
    }
    let diff = |a: &Option<Vec<u8>>, b: &Option<Vec<u8>>| match (a, b) {
        (Some(a), Some(b)) => Some(a.chunks(4).zip(b.chunks(4)).filter(|(x, y)| x[..3] != y[..3]).count()),
        _ => None,
    };
    println!("diff f1-f2 pixels={:?} f1-f3 pixels={:?} total={}", diff(&shots[0], &shots[1]), diff(&shots[0], &shots[2]), w * h);
    sess.destroy();
    src.destroy();
    c.q.roundtrip(&mut c.st).unwrap();
}

fn cmd_screencopy(out: &str) {
    let mut c = connect();
    let shm: wl_shm::WlShm = c.g.bind(&c.qh, 1..=1, ()).unwrap();
    let output: wl_output::WlOutput = c.g.bind(&c.qh, 1..=4, ()).unwrap();
    let sm: ZwlrScreencopyManagerV1 = c.g.bind(&c.qh, 1..=3, ()).expect("zwlr_screencopy_manager_v1");
    let fr = sm.capture_output(0, &output, &c.qh, ());
    let ok = wait(&mut c.q, &mut c.st, 2000, |s| s.sc.buffer_done || s.sc.failed);
    let sc = &c.st.sc;
    println!("screencopy buffer ok={ok} fmt={} {}x{} stride={}", sc.format, sc.w, sc.h, sc.stride);
    let (w, h, stride, fmt) = (sc.w, sc.h, sc.stride, sc.format);
    let (mut file, buf) = shm_buffer(&shm, &c.qh, w, h, stride, fmt);
    fr.copy(&buf);
    let ok = wait(&mut c.q, &mut c.st, 3000, |s| s.sc.ready || s.sc.failed);
    println!("screencopy ready={ok} failed={} flags={}", c.st.sc.failed, c.st.sc.flags);
    if c.st.sc.ready {
        write_png(out, w, h, stride, &read_all(&mut file), c.st.sc.flags & 1 == 1);
    }
}

fn cmd_cursor(points: &[(i32, i32)], secs: u64) {
    let mut c = connect();
    let shm: wl_shm::WlShm = c.g.bind(&c.qh, 1..=1, ()).unwrap();
    let comp: wl_compositor::WlCompositor = c.g.bind(&c.qh, 1..=6, ()).unwrap();
    let ls: ZwlrLayerShellV1 = c.g.bind(&c.qh, 1..=5, ()).expect("zwlr_layer_shell_v1");
    let size = 32u32;
    let mut surfaces = vec![];
    for (i, (x, y)) in points.iter().enumerate() {
        let s = comp.create_surface(&c.qh, ());
        let l = ls.get_layer_surface(&s, None, lsh::Layer::Overlay, "agent-desktop-cursor".into(), &c.qh, i);
        l.set_size(size, size);
        l.set_anchor(lsf::Anchor::Top | lsf::Anchor::Left);
        l.set_margin(*y, 0, 0, *x);
        l.set_exclusive_zone(-1);
        l.set_keyboard_interactivity(lsf::KeyboardInteractivity::None);
        let r = comp.create_region(&c.qh, ());
        s.set_input_region(Some(&r));
        r.destroy();
        s.commit();
        surfaces.push((s, l));
    }
    let colors: [u32; 2] = [0xffff0000, 0xff0000ff];
    let mut buffers = vec![];
    let end = Instant::now() + Duration::from_secs(secs);
    let mut announced = false;
    while Instant::now() < end && !c.st.layer_closed {
        pump(&mut c.q, &mut c.st, 200);
        for (i, serial, _, _) in std::mem::take(&mut c.st.configures) {
            let (s, l) = &surfaces[i];
            l.ack_configure(serial);
            let (mut f, b) = shm_buffer(&shm, &c.qh, size, size, size * 4, 0);
            let px: Vec<u8> = (0..size * size).flat_map(|_| colors[i % 2].to_le_bytes()).collect();
            f.write_all(&px).unwrap();
            s.attach(Some(&b), 0, 0);
            s.damage_buffer(0, 0, size as i32, size as i32);
            s.commit();
            buffers.push((f, b));
        }
        if !announced && buffers.len() == points.len() {
            c.q.roundtrip(&mut c.st).unwrap();
            println!("cursor ready pid={} surfaces={}", std::process::id(), buffers.len());
            std::io::stdout().flush().unwrap();
            announced = true;
        }
    }
    println!("cursor exit closed={}", c.st.layer_closed);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |i: usize| a.get(i).map(String::as_str);
    match arg(1) {
        Some("list") => cmd_list(),
        Some("activate") => cmd_activate(arg(2).unwrap()),
        Some("type") => cmd_type(arg(2).unwrap(), arg(3).map(|d| d.parse().unwrap()).unwrap_or(20)),
        Some("capture") => cmd_capture(arg(2).unwrap(), arg(3).unwrap(), arg(4).filter(|s| !s.is_empty()), arg(5).map(|d| d.parse().unwrap()).unwrap_or(2000)),
        Some("rawkeys") => cmd_rawkeys(arg(2).unwrap(), &a[3..].iter().map(|k| k.parse().unwrap()).collect::<Vec<u32>>()),
        Some("screencopy") => cmd_screencopy(arg(2).unwrap()),
        Some("cursor") => {
            let secs = arg(2).unwrap().parse().unwrap();
            let pts: Vec<(i32, i32)> = a[3..].iter().map(|p| { let (x, y) = p.split_once(',').unwrap(); (x.parse().unwrap(), y.parse().unwrap()) }).collect();
            cmd_cursor(&pts, secs)
        }
        _ => eprintln!("usage: wlprobe list | activate ADDR | type TEXT [delay_ms] | capture ADDR OUTPREFIX [BETWEEN_CMD] [timeout_ms] | screencopy OUT.png | cursor SECS X,Y..."),
    }
}
