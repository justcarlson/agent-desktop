use agent_desktop_core::ErrorCode;

use super::*;
use crate::system::wayland::capture::shm_layout::XRGB8888;

const XBGR8888: u32 = 0x3432_4258;

fn decode(data: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(data))
        .read_info()
        .unwrap();
    let mut out = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut out).unwrap();
    out
}

fn two_rows(format: u32, inverted: bool) -> RawFrame {
    RawFrame {
        data: vec![
            0x30, 0x20, 0x10, 0xff, 0xaa, 0xaa, 0xaa, 0xaa, 0x60, 0x50, 0x40, 0xff, 0xbb, 0xbb,
            0xbb, 0xbb,
        ],
        format,
        width: 1,
        height: 2,
        stride: 8,
        inverted,
    }
}

#[test]
fn rows_are_read_by_stride_and_channels_by_format() {
    let image = frame_image(&two_rows(XRGB8888, false), 1.0).unwrap();
    assert_eq!(
        decode(&image.data),
        vec![0x10, 0x20, 0x30, 0x40, 0x50, 0x60]
    );
    let swapped = frame_image(&two_rows(XBGR8888, false), 1.0).unwrap();
    assert_eq!(
        decode(&swapped.data),
        vec![0x30, 0x20, 0x10, 0x60, 0x50, 0x40]
    );
}

#[test]
fn an_inverted_frame_is_put_upright() {
    let image = frame_image(&two_rows(XRGB8888, true), 1.0).unwrap();
    assert_eq!(
        decode(&image.data),
        vec![0x40, 0x50, 0x60, 0x10, 0x20, 0x30]
    );
}

#[test]
fn the_scale_is_reported_on_the_image() {
    let image = frame_image(&two_rows(XRGB8888, false), 1.5).unwrap();
    assert_eq!((image.width, image.height, image.scale_factor), (1, 2, 1.5));
}

#[test]
fn an_unknown_format_is_refused() {
    let error = frame_image(&two_rows(0x3231_5258, false), 1.0).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
}

#[test]
fn the_scale_is_buffer_pixels_over_logical_width() {
    assert_eq!(scale_of(1500, 1000.0), 1.5);
    assert_eq!(scale_of(800, 800.0), 1.0);
    assert_eq!(scale_of(800, 0.0), 1.0);
    assert_eq!(scale_of(800, f64::NAN), 1.0);
}
