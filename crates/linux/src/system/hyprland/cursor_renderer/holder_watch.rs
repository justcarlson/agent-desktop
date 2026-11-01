//! The renderer's side of hide holders. `hide` answers only after a clear
//! frame was committed and the compositor handled it; `show` draws again
//! only once no holder is live, so one caller's `show` never ends another
//! caller's hide. Each idle tick rereads the holders: a holder that appeared
//! without a `hide` clears the cursor, and the last holder leaving without a
//! `show` draws it again.

use std::path::Path;

use agent_desktop_core::{AdapterError, ErrorCode};

use super::holder_view::{any_live, live_holders};

/// What the watch asks of the renderer's surfaces.
pub(crate) trait HolderCanvas {
    /// Commits transparent content on every cursor surface.
    fn clear_all(&mut self) -> Result<(), AdapterError>;
    /// Waits until the compositor handled every request sent so far.
    fn sync(&mut self) -> Result<(), AdapterError>;
    /// Draws the cursor again where it is, unless it is hidden.
    fn redraw(&mut self) -> Result<(), AdapterError>;
}

/// A holder refusal: the reply kind and its error.
pub(crate) type HolderFault = (&'static str, Box<AdapterError>);

/// Whether the renderer last left its cursors suppressed by a holder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HolderWatch {
    suppressed: bool,
}

impl HolderWatch {
    /// The watch for a renderer that starts with the holders in `holders`;
    /// a renderer started under a live holder draws nothing until it leaves.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn loaded(holders: &Path) -> Self {
        Self {
            suppressed: any_live(holders),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_suppressed(self) -> bool {
        self.suppressed
    }

    /// Follows the holders in `holders` on an idle tick.
    pub(crate) fn refresh(
        &mut self,
        holders: &Path,
        canvas: &mut dyn HolderCanvas,
    ) -> Result<(), AdapterError> {
        let live = any_live(holders);
        if live == self.suppressed {
            return Ok(());
        }
        if live {
            canvas.clear_all()?;
        } else {
            canvas.sync()?;
            canvas.redraw()?;
        }
        self.suppressed = live;
        Ok(())
    }

    /// Hides the cursors for the live holder `holder` and answers after the
    /// compositor handled the clear frame.
    pub(crate) fn hide(
        &mut self,
        holders: &Path,
        holder: &str,
        canvas: &mut dyn HolderCanvas,
    ) -> Result<(), HolderFault> {
        if !live_holders(holders)
            .iter()
            .any(|live| live.nonce == holder)
        {
            let error = AdapterError::new(
                ErrorCode::InvalidArgs,
                format!("Hide holder {holder} is not live in the registry"),
            );
            return Err(("cursor_holder_not_live", Box::new(error)));
        }
        self.suppressed = true;
        let drawn = canvas.clear_all().and_then(|()| canvas.sync());
        drawn.map_err(|error| ("cursor_renderer_draw_failed", Box::new(error)))
    }

    /// Draws the cursors again once no holder is live.
    pub(crate) fn show(
        &mut self,
        holders: &Path,
        canvas: &mut dyn HolderCanvas,
    ) -> Result<(), HolderFault> {
        if any_live(holders) {
            return Ok(());
        }
        self.suppressed = false;
        canvas
            .redraw()
            .map_err(|error| ("cursor_renderer_draw_failed", Box::new(error)))
    }
}

#[cfg(test)]
#[path = "holder_watch_tests.rs"]
mod tests;
