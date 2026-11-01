//! One caller's hide of every agent cursor on a compositor. Taking it writes
//! the holder first, so a renderer that starts afterwards draws nothing;
//! then every renderer in the registry acknowledges `hide`, and the layer
//! stack proves that no other cursor-namespace layer is on screen. Releasing
//! it removes the holder and sends `show`, and each renderer draws again
//! once no holder is live.

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline};

use super::exe_identity::ExeIdentity;
use super::hide_broadcast::{HIDE_DEADLINE_MS, hide_every, hide_one, show_every};
use super::hide_refusal::HideRefusal;
use super::hold_registry::{publish, withdraw};
use super::hold_renewal::{HoldRenewal, RenewTimings};
use super::holder_file::now_ms;
use super::holder_lease::{HolderLease, expiry_for};
use super::layer_audit::audit_cursor_layers;
use super::record_class::RecordClass;
use super::renderer_census::census;
use super::renderer_paths::RendererPaths;
use super::renderer_record::RendererRecord;
use super::renderer_request::RendererRequest;
use crate::system::hyprland::receiver::layer_info::LayerInfo;

/// Reads the compositor's layer surfaces within a deadline.
pub(crate) type LayerRead<'a> = &'a mut dyn FnMut(Deadline) -> Result<Vec<LayerInfo>, AdapterError>;

/// A hide this caller holds until [`HideHold::release`] or drop.
pub(crate) struct HideHold {
    dir: PathBuf,
    nonce: String,
    lease: Arc<Mutex<HolderLease>>,
    renewal: Option<HoldRenewal>,
    released: bool,
}

impl HideHold {
    /// Hides every agent cursor of the compositor whose registry dir is
    /// `dir`, for work that must end by `deadline`. An error is a
    /// not-delivered refusal; nothing stays hidden after it.
    pub(crate) fn take(
        dir: &Path,
        deadline: Deadline,
        timings: RenewTimings,
        read_layers: LayerRead<'_>,
    ) -> Result<Self, AdapterError> {
        let holders = dir.join("holders");
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&holders)
            .map_err(|error| holder_failed(&error.to_string()))?;
        let now = now_ms();
        let work_end = now.saturating_add(deadline_ms(deadline));
        let expiry = expiry_for(work_end, now, timings.cap_ms);
        let lease =
            HolderLease::take(&holders, expiry).map_err(|error| holder_failed(&error.message))?;
        let nonce = lease.nonce().to_owned();
        let mut hold = Self {
            dir: dir.to_path_buf(),
            nonce: nonce.clone(),
            lease: Arc::new(Mutex::new(lease)),
            renewal: None,
            released: false,
        };
        let request = RendererRequest::Hide {
            holder: nonce.clone(),
            expiry_ms: expiry,
        };
        let acked =
            hide_all(dir, &request, deadline, read_layers).map_err(|refusal| refusal.to_error())?;
        publish(&nonce, acked);
        if expiry < work_end {
            hold.renewal = Some(HoldRenewal::start(
                Arc::clone(&hold.lease),
                work_end,
                timings,
            ));
        }
        Ok(hold)
    }

    /// Removes the holder and asks every renderer to draw again. Later calls
    /// do nothing.
    pub(crate) fn release(&mut self) {
        if std::mem::replace(&mut self.released, true) {
            return;
        }
        if let Some(mut renewal) = self.renewal.take() {
            renewal.stop();
        }
        withdraw(&self.nonce);
        self.lease
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .release();
        let show = RendererRequest::Show {
            holder: self.nonce.clone(),
        };
        show_every(&valid_renderers(&self.dir), &show, hide_window(None));
    }
}

impl Drop for HideHold {
    fn drop(&mut self) {
        self.release();
    }
}

fn hide_all(
    dir: &Path,
    request: &RendererRequest,
    deadline: Deadline,
    read_layers: LayerRead<'_>,
) -> Result<Vec<u32>, HideRefusal> {
    let renderers = census(dir, ExeIdentity::current());
    let valid = valid_or_refuse(renderers)?;
    let mut acked = hide_every(&valid, request, hide_window(Some(deadline)))?;
    let window = Duration::from_millis(HIDE_DEADLINE_MS);
    let layers = read_layers(deadline.capped(window)).map_err(|error| HideRefusal::Failed {
        cause: "layer_stack",
        detail: error.message,
    })?;
    let mut hide_late = |pid: u32| late_hide(dir, pid, request, deadline);
    audit_cursor_layers(&layers, &mut acked, &mut hide_late)?;
    Ok(acked)
}

fn late_hide(
    dir: &Path,
    pid: u32,
    request: &RendererRequest,
    deadline: Deadline,
) -> Result<bool, HideRefusal> {
    for (paths, class) in census(dir, ExeIdentity::current()) {
        match class {
            RecordClass::Valid(record) if record.owner.pid == pid => {
                return hide_one(&paths, &record, request, hide_window(Some(deadline)));
            }
            RecordClass::IncompatibleLive(record) if record.owner.pid == pid => {
                return Err(HideRefusal::Incompatible { pid });
            }
            _ => {}
        }
    }
    Ok(false)
}

fn valid_or_refuse(
    renderers: Vec<(RendererPaths, RecordClass)>,
) -> Result<Vec<(RendererPaths, RendererRecord)>, HideRefusal> {
    let mut valid = Vec::new();
    for (paths, class) in renderers {
        match class {
            RecordClass::Valid(record) => valid.push((paths, record)),
            RecordClass::IncompatibleLive(record) => {
                return Err(HideRefusal::Incompatible {
                    pid: record.owner.pid,
                });
            }
            RecordClass::Stale(_) => {}
        }
    }
    Ok(valid)
}

fn valid_renderers(dir: &Path) -> Vec<(RendererPaths, RendererRecord)> {
    census(dir, ExeIdentity::current())
        .into_iter()
        .filter_map(|(paths, class)| match class {
            RecordClass::Valid(record) => Some((paths, record)),
            RecordClass::IncompatibleLive(_) | RecordClass::Stale(_) => None,
        })
        .collect()
}

/// The end of a hide or show round: [`HIDE_DEADLINE_MS`] from now, or the
/// command's deadline when that comes first.
fn hide_window(deadline: Option<Deadline>) -> Instant {
    let window = Duration::from_millis(HIDE_DEADLINE_MS);
    Instant::now() + deadline.map_or(window, |deadline| deadline.remaining().min(window))
}

fn deadline_ms(deadline: Deadline) -> u64 {
    u64::try_from(deadline.remaining().as_millis()).unwrap_or(u64::MAX)
}

fn holder_failed(detail: &str) -> AdapterError {
    HideRefusal::Failed {
        cause: "holder",
        detail: detail.to_owned(),
    }
    .to_error()
}

#[cfg(test)]
#[path = "hide_hold_tests.rs"]
mod tests;
