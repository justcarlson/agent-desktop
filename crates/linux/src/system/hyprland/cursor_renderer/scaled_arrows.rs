//! Arrows drawn for each output scale. The arrow a cursor already holds is
//! its scale-1 drawing; other scales draw it at the style's size times the
//! scale, and a few recent drawings are kept so a travel redraws none.

use agent_desktop_core::CursorOverlayStyle;

use super::agent_cursor::AgentCursor;
use super::arrow_bitmap::ArrowBitmap;

const SCALE_STEP: f64 = 120.0;
const KEPT: usize = 8;

/// Recent arrows by style and scale in 120ths.
#[derive(Default)]
pub(crate) struct ScaledArrows {
    drawn: Vec<(CursorOverlayStyle, i64, ArrowBitmap)>,
}

impl ScaledArrows {
    /// `cursor`'s arrow drawn for an output at `scale`.
    pub(crate) fn at(&mut self, cursor: &AgentCursor, scale: f64) -> ArrowBitmap {
        let steps = (scale * SCALE_STEP).round() as i64;
        if steps == SCALE_STEP as i64 || steps <= 0 {
            return cursor.bitmap.clone();
        }
        if let Some((_, _, bitmap)) = self
            .drawn
            .iter()
            .find(|(style, kept, _)| *kept == steps && *style == cursor.style)
        {
            return bitmap.clone();
        }
        let mut style = cursor.style.clone();
        style.set_size(cursor.style.size() * steps as f64 / SCALE_STEP);
        let bitmap = ArrowBitmap::draw(&style);
        if self.drawn.len() >= KEPT {
            self.drawn.remove(0);
        }
        self.drawn
            .push((cursor.style.clone(), steps, bitmap.clone()));
        bitmap
    }
}
