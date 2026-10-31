use super::*;
use crate::system::capture::pixels::to_rgb;

const ARGB8888: u32 = 0;
const XRGB8888: u32 = 1;
const XBGR8888: u32 = 0x3432_4258;
const ABGR8888: u32 = 0x3432_4241;
const RGB565: u32 = 0x3631_5652;

#[test]
fn an_opaque_little_endian_format_is_preferred_over_one_with_alpha() {
    assert_eq!(choose_format(&[ARGB8888, XRGB8888]), Some(XRGB8888));
    assert_eq!(choose_format(&[ABGR8888, XBGR8888]), Some(XBGR8888));
    assert_eq!(choose_format(&[ABGR8888, ARGB8888]), Some(ARGB8888));
}

#[test]
fn a_format_without_eight_bit_channels_is_never_chosen() {
    assert_eq!(choose_format(&[RGB565]), None);
    assert_eq!(choose_format(&[]), None);
    assert!(visual_of(RGB565).is_none());
}

#[test]
fn xrgb_bytes_in_memory_are_blue_green_red_pad() {
    let visual = visual_of(XRGB8888).unwrap();
    let rgb = to_rgb(&visual, &[3, 2, 1, 0, 6, 5, 4, 0xff], 2, 1, 8).unwrap();
    assert_eq!(rgb, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn xbgr_bytes_in_memory_are_red_green_blue_pad() {
    let visual = visual_of(XBGR8888).unwrap();
    let rgb = to_rgb(&visual, &[1, 2, 3, 0, 4, 5, 6, 0xff], 2, 1, 8).unwrap();
    assert_eq!(rgb, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn an_inverted_frame_is_turned_upright_row_by_row() {
    let rgb = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6];
    let upright = upright(rgb.clone(), 1, 6, true);
    assert_eq!(
        upright,
        vec![6, 6, 6, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1]
    );
    assert_eq!(super::upright(rgb.clone(), 1, 6, false), rgb);
    let wide = vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4];
    assert_eq!(
        super::upright(wide, 2, 2, true),
        vec![3, 3, 3, 4, 4, 4, 1, 1, 1, 2, 2, 2]
    );
}

#[test]
fn a_buffer_size_is_the_stride_times_the_height_or_nothing() {
    assert_eq!(buffer_len(4, 3), Some((16, 48)));
    assert_eq!(buffer_len(0, 3), None);
    assert_eq!(buffer_len(4, 0), None);
    assert_eq!(buffer_len(u32::MAX, u32::MAX), None);
}
