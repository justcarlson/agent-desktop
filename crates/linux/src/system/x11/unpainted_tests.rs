use super::*;
use crate::system::capture::pixels::VisualFormat;

fn raw(width: u32, height: u32, fill: impl Fn(u32, u32) -> [u8; 3]) -> Raw {
    let mut data = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let [a, b, c] = fill(x, y);
            data.extend_from_slice(&[a, b, c, 0]);
        }
    }
    Raw {
        data,
        width,
        height,
        stride: width as usize * PIXEL_BYTES,
        format: VisualFormat {
            red_mask: 0xff0000,
            green_mask: 0xff00,
            blue_mask: 0xff,
            bits_per_pixel: 32,
            depth: 24,
            scanline_pad: 32,
            msb_first: false,
        },
    }
}

const WHOLE: Region = Region {
    x: 0,
    y: 0,
    width: 4,
    height: 3,
};

#[test]
fn a_pixmap_equal_to_the_screen_in_the_region_is_a_match() {
    let screen = raw(4, 3, |_, _| [1, 2, 3]);
    let pixmap = raw(4, 3, |_, _| [1, 2, 3]);
    assert!(matches_screen(&screen, &pixmap, &[WHOLE]));
}

#[test]
fn one_pixel_that_differs_in_the_region_is_no_match() {
    let screen = raw(4, 3, |_, _| [1, 2, 3]);
    let pixmap = raw(4, 3, |x, y| {
        if (x, y) == (2, 1) {
            [9, 9, 9]
        } else {
            [1, 2, 3]
        }
    });
    assert!(!matches_screen(&screen, &pixmap, &[WHOLE]));
}

#[test]
fn a_difference_outside_every_region_is_not_looked_at() {
    let screen = raw(4, 3, |_, _| [1, 2, 3]);
    let pixmap = raw(4, 3, |x, _| if x == 0 { [9, 9, 9] } else { [1, 2, 3] });
    let content = Region {
        x: 1,
        y: 0,
        width: 3,
        height: 3,
    };
    assert!(matches_screen(&screen, &pixmap, &[content]));
}

#[test]
fn the_padding_byte_is_not_compared() {
    let screen = raw(4, 3, |_, _| [1, 2, 3]);
    let mut pixmap = raw(4, 3, |_, _| [1, 2, 3]);
    pixmap.data[3] = 0xff;
    assert!(matches_screen(&screen, &pixmap, &[WHOLE]));
}

#[test]
fn a_region_with_no_pixel_inside_the_images_is_no_match() {
    let screen = raw(4, 3, |_, _| [1, 2, 3]);
    let pixmap = raw(4, 3, |_, _| [1, 2, 3]);
    let outside = Region {
        x: 10,
        y: 10,
        width: 5,
        height: 5,
    };
    assert!(!matches_screen(&screen, &pixmap, &[outside]));
    assert!(!matches_screen(&screen, &pixmap, &[]));
}

#[test]
fn a_region_reaching_past_the_images_is_clipped() {
    let screen = raw(4, 3, |_, _| [1, 2, 3]);
    let pixmap = raw(4, 3, |_, _| [1, 2, 3]);
    let past = Region {
        x: -2,
        y: -2,
        width: 100,
        height: 100,
    };
    assert!(matches_screen(&screen, &pixmap, &[past]));
}
