//! The globals a capture binds, chosen from the registry at the lower of
//! the advertised version and the version this client speaks. A global
//! below the version a capture needs counts as missing, and the first
//! missing one is named.

use super::output_capture_globals::OutputCaptureGlobals;
use super::toplevel_capture_globals::ToplevelCaptureGlobals;
use crate::system::wayland::toplevel::advertised_global::AdvertisedGlobal;
use crate::system::wayland::toplevel::chosen_globals::pick;

const SHM: (&str, u32) = ("wl_shm", 1);
const SOURCES: (&str, u32) = ("ext_foreign_toplevel_image_capture_source_manager_v1", 1);
const COPY: (&str, u32) = ("ext_image_copy_capture_manager_v1", 1);
const SCREENCOPY: (&str, u32) = ("zwlr_screencopy_manager_v1", 3);
const OUTPUT: (&str, u32) = ("wl_output", 4);

pub(crate) fn toplevel(
    globals: &[AdvertisedGlobal],
) -> Result<ToplevelCaptureGlobals, &'static str> {
    Ok(ToplevelCaptureGlobals {
        shm: at_least(globals, SHM)?,
        sources: at_least(globals, SOURCES)?,
        copy: at_least(globals, COPY)?,
    })
}

pub(crate) fn output(globals: &[AdvertisedGlobal]) -> Result<OutputCaptureGlobals, &'static str> {
    let outputs: Vec<AdvertisedGlobal> = globals
        .iter()
        .filter(|global| global.interface == OUTPUT.0 && global.version >= OUTPUT.1)
        .map(|global| AdvertisedGlobal {
            version: OUTPUT.1,
            ..global.clone()
        })
        .collect();
    let shm = at_least(globals, SHM)?;
    let screencopy = at_least(globals, SCREENCOPY)?;
    if outputs.is_empty() {
        return Err(OUTPUT.0);
    }
    Ok(OutputCaptureGlobals {
        shm,
        screencopy,
        outputs,
    })
}

fn at_least(
    globals: &[AdvertisedGlobal],
    (interface, needs): (&'static str, u32),
) -> Result<AdvertisedGlobal, &'static str> {
    pick(globals, interface, needs)
        .filter(|global| global.version >= needs)
        .ok_or(interface)
}

#[cfg(test)]
#[path = "capture_globals_tests.rs"]
mod tests;
