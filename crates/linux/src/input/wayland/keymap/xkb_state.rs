//! The XKB state a virtual keyboard's keys produce, kept beside the
//! compositor's so every `modifiers` request names the modifiers the keys
//! hold and the layout the delivery types in.

use std::ffi::{c_int, c_void};
use std::ptr::NonNull;

use agent_desktop_core::AdapterError;

use super::key_table::EVDEV_OFFSET;
use super::modifier_state::ModifierState;
use super::refusal::xkb_unavailable;
use super::xkb_keymap::XkbKeymap;
use super::xkb_library::XkbLibrary;

const MODS_DEPRESSED: c_int = 1;
const MODS_LATCHED: c_int = 1 << 1;
const MODS_LOCKED: c_int = 1 << 2;
const LAYOUT_EFFECTIVE: c_int = 1 << 7;
const KEY_UP: c_int = 0;
const KEY_DOWN: c_int = 1;

type StateNew = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type StateUnref = unsafe extern "C" fn(*mut c_void);
type UpdateMask = unsafe extern "C" fn(*mut c_void, u32, u32, u32, u32, u32, u32) -> c_int;
type UpdateKey = unsafe extern "C" fn(*mut c_void, u32, c_int) -> c_int;
type SerializeMods = unsafe extern "C" fn(*mut c_void, c_int) -> u32;
type SerializeLayout = unsafe extern "C" fn(*mut c_void, c_int) -> u32;

/// One `xkb_state` over a compiled keymap. The state holds its own reference
/// to the keymap and the library stays open for as long as the state lives.
pub(crate) struct XkbState {
    xkb: XkbLibrary,
    state: NonNull<c_void>,
}

impl XkbState {
    /// A state with no modifier held and layout `layout` locked.
    pub(crate) fn new(keymap: &XkbKeymap, layout: u32) -> Result<Self, AdapterError> {
        let xkb = XkbLibrary::open()?;
        let state = unsafe {
            let state_new: StateNew = xkb.symbol(c"xkb_state_new")?;
            let update: UpdateMask = xkb.symbol(c"xkb_state_update_mask")?;
            let state = NonNull::new(state_new(keymap.raw()))
                .ok_or_else(|| xkb_unavailable("xkb_state_new failed"))?;
            update(state.as_ptr(), 0, 0, 0, 0, 0, layout);
            state
        };
        Ok(Self { xkb, state })
    }

    /// Applies one evdev key edge and returns the state that follows it.
    pub(crate) fn update_key(
        &mut self,
        evdev_code: u32,
        down: bool,
    ) -> Result<ModifierState, AdapterError> {
        unsafe {
            let update: UpdateKey = self.xkb.symbol(c"xkb_state_update_key")?;
            let direction = if down { KEY_DOWN } else { KEY_UP };
            update(self.state.as_ptr(), evdev_code + EVDEV_OFFSET, direction);
        }
        self.modifiers()
    }

    /// The modifiers and the effective layout as of the last key edge.
    pub(crate) fn modifiers(&self) -> Result<ModifierState, AdapterError> {
        unsafe {
            let mods: SerializeMods = self.xkb.symbol(c"xkb_state_serialize_mods")?;
            let layout: SerializeLayout = self.xkb.symbol(c"xkb_state_serialize_layout")?;
            let state = self.state.as_ptr();
            Ok(ModifierState {
                depressed: mods(state, MODS_DEPRESSED),
                latched: mods(state, MODS_LATCHED),
                locked: mods(state, MODS_LOCKED),
                group: layout(state, LAYOUT_EFFECTIVE),
            })
        }
    }
}

impl Drop for XkbState {
    fn drop(&mut self) {
        if let Ok(unref) = unsafe { self.xkb.symbol::<StateUnref>(c"xkb_state_unref") } {
            unsafe { unref(self.state.as_ptr()) };
        }
    }
}

#[cfg(test)]
#[path = "xkb_state_tests.rs"]
mod tests;
