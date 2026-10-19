use x11rb::protocol::xproto::{Depth, Format, VisualClass, Visualtype};

use super::{find_visual, pixmap_format};

fn visual(id: u32) -> Visualtype {
    Visualtype {
        visual_id: id,
        class: VisualClass::TRUE_COLOR,
        bits_per_rgb_value: 8,
        colormap_entries: 256,
        red_mask: 0xff0000,
        green_mask: 0x00ff00,
        blue_mask: 0x0000ff,
    }
}

#[test]
fn a_visual_is_found_in_any_depth_by_its_id() {
    let depths = [
        Depth {
            depth: 24,
            visuals: vec![visual(0x21)],
        },
        Depth {
            depth: 32,
            visuals: vec![visual(0x22), visual(0x23)],
        },
    ];
    assert_eq!(find_visual(depths.iter(), 0x23).unwrap().visual_id, 0x23);
    assert!(find_visual(depths.iter(), 0x99).is_none());
}

#[test]
fn the_pixmap_format_is_the_one_of_the_depth() {
    let formats = [
        Format {
            depth: 24,
            bits_per_pixel: 32,
            scanline_pad: 32,
        },
        Format {
            depth: 1,
            bits_per_pixel: 1,
            scanline_pad: 32,
        },
    ];
    assert_eq!(pixmap_format(&formats, 24).unwrap().bits_per_pixel, 32);
    assert_eq!(pixmap_format(&formats, 1).unwrap().bits_per_pixel, 1);
    assert!(pixmap_format(&formats, 16).is_none());
}
