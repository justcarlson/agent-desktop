//! The one cursor a renderer draws: its style, motion profile, pose and the
//! arrow drawn for its style. Per-agent cursors and profiles are not kept
//! here yet; every control updates the same cursor.

use agent_desktop_core::{CursorMotionProfile, CursorOverlayControl, CursorOverlayStyle, Point};

use super::arrow_bitmap::ArrowBitmap;

/// One cursor's state.
pub(crate) struct AgentCursor {
    pub(crate) style: CursorOverlayStyle,
    pub(crate) motion: CursorMotionProfile,
    pub(crate) pose: Option<Point>,
    pub(crate) moves: u64,
    pub(crate) bitmap: ArrowBitmap,
    pub(crate) hidden: bool,
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
            hidden: false,
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
