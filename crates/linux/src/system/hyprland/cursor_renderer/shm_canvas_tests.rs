use super::compose;
use crate::system::hyprland::cursor_renderer::arrow_bitmap::ArrowBitmap;

fn bitmap() -> ArrowBitmap {
    ArrowBitmap {
        width: 2,
        height: 2,
        pixels: (1..=16).collect(),
        hotspot: (1.0, 1.0),
    }
}

#[test]
fn the_arrow_lands_at_its_offset_on_a_clear_frame() {
    let frame = compose(&bitmap(), (4, 3), (1, 1));
    let pixel = |x: usize, y: usize| frame[(y * 4 + x) * 4];
    assert_eq!(pixel(0, 0), 0);
    assert_eq!(pixel(1, 1), 1);
    assert_eq!(pixel(2, 1), 5);
    assert_eq!(pixel(1, 2), 9);
    assert_eq!(pixel(3, 2), 0);
}

#[test]
fn an_arrow_wider_than_the_frame_is_cut_not_wrapped() {
    let frame = compose(&bitmap(), (2, 2), (1, 1));
    assert_eq!(frame.len(), 16);
    assert_eq!(&frame[12..16], &[1, 2, 3, 4]);
    assert_eq!(&frame[0..12], &[0; 12]);
}
