use agent_desktop_core::CursorOverlayStyle;

use super::ArrowBitmap;

fn pixel(bitmap: &ArrowBitmap, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * bitmap.width + x) * 4) as usize;
    bitmap.pixels[at..at + 4].try_into().expect("four bytes")
}

#[test]
fn the_default_arrow_fits_its_box_with_a_transparent_corner() {
    let bitmap = ArrowBitmap::draw(&CursorOverlayStyle::default());
    assert_eq!(
        bitmap.pixels.len(),
        (bitmap.width * bitmap.height * 4) as usize
    );
    assert_eq!(bitmap.stride(), bitmap.width * 4);
    assert_eq!(pixel(&bitmap, bitmap.width - 1, 0), [0, 0, 0, 0]);
    assert_eq!(bitmap.hotspot, (1.0, 1.0));
}

#[test]
fn the_body_is_opaque_and_premultiplied() {
    let bitmap = ArrowBitmap::draw(&CursorOverlayStyle::default());
    let body = pixel(&bitmap, 6, 14);
    assert_eq!(body[3], 255);
    assert!(
        bitmap
            .pixels
            .chunks_exact(4)
            .all(|p| p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3])
    );
}

#[test]
fn a_larger_size_draws_a_larger_box() {
    let mut style = CursorOverlayStyle::default();
    let small = ArrowBitmap::draw(&style);
    style.set_size(2.0);
    let large = ArrowBitmap::draw(&style);
    assert!(large.width > small.width && large.height > small.height);
}
