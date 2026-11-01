use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::ops::RangeInclusive;
use std::ptr::NonNull;

use agent_desktop_core::AdapterError;

use super::refusal::xkb_unavailable;
use super::xkb_library::XkbLibrary;

const CONTEXT_NO_DEFAULT_INCLUDES: c_int = 1;
const CONTEXT_NO_ENVIRONMENT_NAMES: c_int = 2;
const LOG_LEVEL_CRITICAL: c_int = 10;
const KEYMAP_FORMAT_TEXT_V1: c_int = 1;
const KEYCODE_INVALID: u32 = u32::MAX;
const MAX_MASKS: usize = 16;
const KEYSYM_NAME_BYTES: usize = 64;

type ContextNew = unsafe extern "C" fn(c_int) -> *mut c_void;
type SetLogLevel = unsafe extern "C" fn(*mut c_void, c_int);
type Unref = unsafe extern "C" fn(*mut c_void);
type KeymapNew =
    unsafe extern "C" fn(*mut c_void, *const c_char, usize, c_int, c_int) -> *mut c_void;
type KeycodeBound = unsafe extern "C" fn(*mut c_void) -> u32;
type LayoutsForKey = unsafe extern "C" fn(*mut c_void, u32) -> u32;
type LevelsForKey = unsafe extern "C" fn(*mut c_void, u32, u32) -> u32;
type SymsByLevel = unsafe extern "C" fn(*mut c_void, u32, u32, u32, *mut *const u32) -> c_int;
type ModsForLevel = unsafe extern "C" fn(*mut c_void, u32, u32, u32, *mut u32, usize) -> usize;
type ModIndex = unsafe extern "C" fn(*mut c_void, *const c_char) -> u32;
type KeyName = unsafe extern "C" fn(*mut c_void, u32) -> *const c_char;
type KeyByName = unsafe extern "C" fn(*mut c_void, *const c_char) -> u32;
type KeysymName = unsafe extern "C" fn(u32, *mut c_char, usize) -> c_int;
type KeysymUtf32 = unsafe extern "C" fn(u32) -> u32;
type NumLayouts = unsafe extern "C" fn(*mut c_void) -> u32;
type LayoutName = unsafe extern "C" fn(*mut c_void, u32) -> *const c_char;

/// One compiled XKB keymap. It holds the library open for as long as the
/// keymap lives, so every call below reads valid memory.
pub(crate) struct XkbKeymap {
    xkb: XkbLibrary,
    keymap: NonNull<c_void>,
}

impl XkbKeymap {
    /// Compiles XKB keymap text. `None` means libxkbcommon rejected it.
    pub(crate) fn compile(text: &[u8]) -> Result<Option<Self>, AdapterError> {
        let xkb = XkbLibrary::open()?;
        let keymap = unsafe {
            let context_new: ContextNew = xkb.symbol(c"xkb_context_new")?;
            let set_log_level: SetLogLevel = xkb.symbol(c"xkb_context_set_log_level")?;
            let context_unref: Unref = xkb.symbol(c"xkb_context_unref")?;
            let keymap_new: KeymapNew = xkb.symbol(c"xkb_keymap_new_from_buffer")?;
            let context = context_new(CONTEXT_NO_DEFAULT_INCLUDES | CONTEXT_NO_ENVIRONMENT_NAMES);
            if context.is_null() {
                return Err(xkb_unavailable("xkb_context_new failed"));
            }
            set_log_level(context, LOG_LEVEL_CRITICAL);
            let keymap = keymap_new(
                context,
                text.as_ptr().cast(),
                text.len(),
                KEYMAP_FORMAT_TEXT_V1,
                0,
            );
            context_unref(context);
            keymap
        };
        Ok(NonNull::new(keymap).map(|keymap| Self { xkb, keymap }))
    }

    pub(super) fn raw(&self) -> *mut c_void {
        self.keymap.as_ptr()
    }

    pub(crate) fn keycodes(&self) -> Result<RangeInclusive<u32>, AdapterError> {
        unsafe {
            let min: KeycodeBound = self.xkb.symbol(c"xkb_keymap_min_keycode")?;
            let max: KeycodeBound = self.xkb.symbol(c"xkb_keymap_max_keycode")?;
            Ok(min(self.raw())..=max(self.raw()))
        }
    }

    pub(crate) fn layouts_for_key(&self, keycode: u32) -> Result<u32, AdapterError> {
        unsafe {
            let layouts: LayoutsForKey = self.xkb.symbol(c"xkb_keymap_num_layouts_for_key")?;
            Ok(layouts(self.raw(), keycode))
        }
    }

    pub(crate) fn levels_for_key(&self, keycode: u32, layout: u32) -> Result<u32, AdapterError> {
        unsafe {
            let levels: LevelsForKey = self.xkb.symbol(c"xkb_keymap_num_levels_for_key")?;
            Ok(levels(self.raw(), keycode, layout))
        }
    }

    pub(crate) fn syms_by_level(
        &self,
        keycode: u32,
        layout: u32,
        level: u32,
    ) -> Result<Vec<u32>, AdapterError> {
        unsafe {
            let syms: SymsByLevel = self.xkb.symbol(c"xkb_keymap_key_get_syms_by_level")?;
            let mut first: *const u32 = std::ptr::null();
            let count = syms(self.raw(), keycode, layout, level, &raw mut first);
            let count = usize::try_from(count).unwrap_or(0);
            if first.is_null() || count == 0 {
                return Ok(Vec::new());
            }
            Ok(std::slice::from_raw_parts(first, count).to_vec())
        }
    }

    pub(crate) fn masks_for_level(
        &self,
        keycode: u32,
        layout: u32,
        level: u32,
    ) -> Result<Vec<u32>, AdapterError> {
        unsafe {
            let masks: ModsForLevel = self.xkb.symbol(c"xkb_keymap_key_get_mods_for_level")?;
            let mut found = [0_u32; MAX_MASKS];
            let written = masks(
                self.raw(),
                keycode,
                layout,
                level,
                found.as_mut_ptr(),
                MAX_MASKS,
            );
            Ok(found[..written.min(MAX_MASKS)].to_vec())
        }
    }

    /// The name of every layout, in layout index order. A layout without a
    /// name reads as an empty string.
    pub(crate) fn layout_names(&self) -> Result<Vec<String>, AdapterError> {
        unsafe {
            let count: NumLayouts = self.xkb.symbol(c"xkb_keymap_num_layouts")?;
            let name: LayoutName = self.xkb.symbol(c"xkb_keymap_layout_get_name")?;
            Ok((0..count(self.raw()))
                .map(|layout| {
                    let found = name(self.raw(), layout);
                    if found.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(found).to_string_lossy().into_owned()
                    }
                })
                .collect())
        }
    }

    /// The modifier mask of the named modifier, or 0 when the keymap has none.
    pub(crate) fn mod_mask(&self, name: &CStr) -> Result<u32, AdapterError> {
        unsafe {
            let index: ModIndex = self.xkb.symbol(c"xkb_keymap_mod_get_index")?;
            Ok(1_u32
                .checked_shl(index(self.raw(), name.as_ptr()))
                .unwrap_or(0))
        }
    }

    pub(crate) fn key_name(&self, keycode: u32) -> Result<Option<String>, AdapterError> {
        unsafe {
            let name: KeyName = self.xkb.symbol(c"xkb_keymap_key_get_name")?;
            let found = name(self.raw(), keycode);
            if found.is_null() {
                return Ok(None);
            }
            Ok(Some(CStr::from_ptr(found).to_string_lossy().into_owned()))
        }
    }

    /// Whether a key or a key alias already uses `name`.
    pub(crate) fn has_key_name(&self, name: &str) -> Result<bool, AdapterError> {
        let Ok(name) = CString::new(name) else {
            return Ok(true);
        };
        unsafe {
            let by_name: KeyByName = self.xkb.symbol(c"xkb_keymap_key_by_name")?;
            Ok(by_name(self.raw(), name.as_ptr()) != KEYCODE_INVALID)
        }
    }

    /// The name XKB text uses for `keysym`, or `None` for an invalid keysym.
    pub(crate) fn keysym_name(&self, keysym: u32) -> Result<Option<String>, AdapterError> {
        let mut buffer = [0 as c_char; KEYSYM_NAME_BYTES];
        let written = unsafe {
            let name: KeysymName = self.xkb.symbol(c"xkb_keysym_get_name")?;
            name(keysym, buffer.as_mut_ptr(), buffer.len())
        };
        let fits = usize::try_from(written).is_ok_and(|length| length > 0 && length < buffer.len());
        if !fits {
            return Ok(None);
        }
        let name = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        Ok(Some(name.to_string_lossy().into_owned()))
    }

    /// The character `keysym` types, or `None` when it types none.
    pub(crate) fn keysym_char(&self, keysym: u32) -> Result<Option<char>, AdapterError> {
        let code = unsafe {
            let to_utf32: KeysymUtf32 = self.xkb.symbol(c"xkb_keysym_to_utf32")?;
            to_utf32(keysym)
        };
        Ok(char::from_u32(code).filter(|character| *character != '\0'))
    }
}

impl Drop for XkbKeymap {
    fn drop(&mut self) {
        if let Ok(unref) = unsafe { self.xkb.symbol::<Unref>(c"xkb_keymap_unref") } {
            unsafe { unref(self.raw()) };
        }
    }
}

#[cfg(test)]
type StateNew = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
#[cfg(test)]
type StateUpdateMask = unsafe extern "C" fn(*mut c_void, u32, u32, u32, u32, u32, u32) -> c_int;
#[cfg(test)]
type StateOneSym = unsafe extern "C" fn(*mut c_void, u32) -> u32;

#[cfg(test)]
impl XkbKeymap {
    /// The keysym an XKB client reads for `keycode` with layout `layout`
    /// locked and only the modifiers in `depressed` held: an independent
    /// read through `xkb_state`, not through the levels the key table walks.
    pub(crate) fn typed_keysym(
        &self,
        keycode: u32,
        layout: u32,
        depressed: u32,
    ) -> Result<u32, AdapterError> {
        unsafe {
            let state_new: StateNew = self.xkb.symbol(c"xkb_state_new")?;
            let state_unref: Unref = self.xkb.symbol(c"xkb_state_unref")?;
            let update: StateUpdateMask = self.xkb.symbol(c"xkb_state_update_mask")?;
            let one_sym: StateOneSym = self.xkb.symbol(c"xkb_state_key_get_one_sym")?;
            let state = state_new(self.raw());
            if state.is_null() {
                return Err(xkb_unavailable("xkb_state_new failed"));
            }
            update(state, depressed, 0, 0, 0, 0, layout);
            let keysym = one_sym(state, keycode);
            state_unref(state);
            Ok(keysym)
        }
    }
}
