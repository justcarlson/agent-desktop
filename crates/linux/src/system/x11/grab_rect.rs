#![cfg_attr(not(test), expect(dead_code, reason = "stub the owning lane replaces"))]

use super::grab_error::GrabError;
use crate::tree::Extents;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GrabRect {
    pub(crate) x: i16,
    pub(crate) y: i16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl GrabRect {
    pub(crate) fn from_extents(extents: Extents) -> Option<Self> {
        Some(Self {
            x: i16::try_from(extents.x).ok()?,
            y: i16::try_from(extents.y).ok()?,
            width: u16::try_from(extents.width)
                .ok()
                .filter(|width| *width > 0)?,
            height: u16::try_from(extents.height)
                .ok()
                .filter(|height| *height > 0)?,
        })
    }

    pub(crate) fn clip_to(self, width: i32, height: i32) -> Option<Self> {
        let left = i32::from(self.x).max(0);
        let top = i32::from(self.y).max(0);
        let right = (i32::from(self.x) + i32::from(self.width)).min(width);
        let bottom = (i32::from(self.y) + i32::from(self.height)).min(height);
        (right > left && bottom > top).then(|| Self {
            x: left as i16,
            y: top as i16,
            width: (right - left) as u16,
            height: (bottom - top) as u16,
        })
    }

    pub(crate) fn strips(self, row_bytes: usize, limit: usize) -> Result<Vec<Self>, GrabError> {
        let rows = limit / row_bytes.max(1);
        if rows == 0 {
            return Err(GrabError::Resource);
        }
        let rows = rows.min(usize::from(self.height));
        let mut strips = Vec::with_capacity(usize::from(self.height).div_ceil(rows));
        let mut done = 0usize;
        while done < usize::from(self.height) {
            let take = rows.min(usize::from(self.height) - done);
            let y =
                i16::try_from(i32::from(self.y) + done as i32).map_err(|_| GrabError::Resource)?;
            strips.push(Self {
                y,
                height: take as u16,
                ..self
            });
            done += take;
        }
        Ok(strips)
    }
}

#[cfg(test)]
#[path = "grab_rect_tests.rs"]
mod tests;
