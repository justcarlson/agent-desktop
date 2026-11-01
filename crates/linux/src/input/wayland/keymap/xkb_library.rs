use std::ffi::{CStr, c_void};
use std::ptr::NonNull;

use agent_desktop_core::AdapterError;

use super::refusal::xkb_unavailable;

const LIBRARY: &CStr = c"libxkbcommon.so.0";

/// libxkbcommon, opened with `dlopen` when a keymap is read, so the binary
/// links nothing new and a session without the library fails only the
/// deliveries that need it.
pub(crate) struct XkbLibrary {
    handle: NonNull<c_void>,
}

impl XkbLibrary {
    pub(crate) fn open() -> Result<Self, AdapterError> {
        let handle = unsafe { libc::dlopen(LIBRARY.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
        NonNull::new(handle)
            .map(|handle| Self { handle })
            .ok_or_else(|| xkb_unavailable("libxkbcommon.so.0 could not be loaded"))
    }

    /// The exported function `name`, as the function pointer type `T`.
    ///
    /// # Safety
    ///
    /// `T` must be the `extern "C"` function pointer type that `name` has in
    /// the libxkbcommon headers.
    pub(crate) unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Result<T, AdapterError> {
        let address = unsafe { libc::dlsym(self.handle.as_ptr(), name.as_ptr()) };
        if address.is_null() {
            return Err(xkb_unavailable(&format!(
                "libxkbcommon exports no {}",
                name.to_string_lossy()
            )));
        }
        Ok(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&address) })
    }
}

impl Drop for XkbLibrary {
    fn drop(&mut self) {
        unsafe {
            libc::dlclose(self.handle.as_ptr());
        }
    }
}
