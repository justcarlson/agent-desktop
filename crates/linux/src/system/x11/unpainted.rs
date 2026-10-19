use super::grab::Raw;

const PIXEL_BYTES: usize = 4;

/// A rectangle inside a grabbed area, in the area's own coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Region {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

fn pixel_at(raw: &Raw, x: usize, y: usize) -> Option<[u8; 3]> {
    let start = y * raw.stride + x * PIXEL_BYTES;
    let bytes = raw.data.get(start..start + 3)?;
    Some([bytes[0], bytes[1], bytes[2]])
}

/// Whether every pixel of the regions that lies inside both images is the
/// same in the two. A redirected pixmap whose window content equals what the
/// screen showed has not been painted by the window: it holds the screen's
/// copy, which can be the pixels of whatever covers the window. A grab with
/// no pixel to compare is not a match.
pub(super) fn matches_screen(screen: &Raw, pixmap: &Raw, regions: &[Region]) -> bool {
    let width = screen.width.min(pixmap.width) as i32;
    let height = screen.height.min(pixmap.height) as i32;
    let mut compared = 0usize;
    for region in regions {
        let left = region.x.max(0);
        let top = region.y.max(0);
        let right = (region.x + region.width).min(width);
        let bottom = (region.y + region.height).min(height);
        for y in top..bottom {
            for x in left..right {
                let (x, y) = (x as usize, y as usize);
                if pixel_at(screen, x, y) != pixel_at(pixmap, x, y) {
                    return false;
                }
                compared += 1;
            }
        }
    }
    compared > 0
}

#[cfg(test)]
#[path = "unpainted_tests.rs"]
mod tests;
