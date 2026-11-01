//! One agent's cursor: its style and motion profile, pose, the arrow drawn
//! for its style, whether the agent hid it or a hidden target parked it,
//! and its travel in flight. A profile changes only how the cursor looks
//! and moves, never where it goes.

use agent_desktop_core::{CursorMotionProfile, CursorOverlayControl, CursorOverlayStyle, Point};

use super::arrow_bitmap::ArrowBitmap;
use super::cursor_concealment::CursorConcealment;
use super::cursor_travel::CursorTravel;

/// One cursor's state.
pub(crate) struct AgentCursor {
    pub(crate) style: CursorOverlayStyle,
    pub(crate) motion: CursorMotionProfile,
    pub(crate) pose: Option<Point>,
    pub(crate) moves: u64,
    pub(crate) bitmap: ArrowBitmap,
    pub(crate) concealed: CursorConcealment,
    pub(crate) travel: Option<CursorTravel>,
}

impl AgentCursor {
    /// A cursor with `control`'s style and motion, not yet placed.
    pub(crate) fn new(control: &CursorOverlayControl) -> Self {
        let style = control.style().cloned().unwrap_or_default();
        Self {
            bitmap: ArrowBitmap::draw(&style),
            motion: control.motion().cloned().unwrap_or_default(),
            style,
            pose: None,
            moves: 0,
            concealed: CursorConcealment::default(),
            travel: None,
        }
    }

    /// Takes the style and motion `control` carries, redrawing the arrow
    /// only when the style changed.
    pub(crate) fn adopt(&mut self, control: &CursorOverlayControl) {
        if let Some(style) = control.style()
            && *style != self.style
        {
            self.style = style.clone();
            self.bitmap = ArrowBitmap::draw(style);
        }
        if let Some(motion) = control.motion() {
            self.motion = motion.clone();
        }
    }
}
