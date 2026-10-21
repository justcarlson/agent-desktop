use x11rb::protocol::xproto::{Depth, Format, ImageOrder, Setup, Visualtype};

use super::channel_masks::masks_of;
use crate::system::capture::pixels::VisualFormat;

pub(crate) fn find_visual<'a>(
    depths: impl Iterator<Item = &'a Depth>,
    id: u32,
) -> Option<&'a Visualtype> {
    depths
        .flat_map(|depth| depth.visuals.iter())
        .find(|visual| visual.visual_id == id)
}

pub(crate) fn pixmap_format(formats: &[Format], depth: u8) -> Option<&Format> {
    formats.iter().find(|format| format.depth == depth)
}

pub(crate) fn format_of(setup: &Setup, depth: u8, visual_id: u32) -> Option<VisualFormat> {
    let visual = find_visual(
        setup
            .roots
            .iter()
            .flat_map(|screen| screen.allowed_depths.iter()),
        visual_id,
    )?;
    let format = pixmap_format(&setup.pixmap_formats, depth)?;
    let [red_mask, green_mask, blue_mask] = masks_of(visual);
    Some(VisualFormat {
        red_mask,
        green_mask,
        blue_mask,
        bits_per_pixel: format.bits_per_pixel,
        depth,
        scanline_pad: format.scanline_pad,
        msb_first: setup.image_byte_order == ImageOrder::MSB_FIRST,
    })
}
