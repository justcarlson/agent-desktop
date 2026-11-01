/// Where one requested keysym is typed under the extended keymap: its XKB
/// keycode, its level (1 plain, 2 with Shift) and whether the extension added
/// the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyPlacement {
    pub(crate) keysym: u32,
    pub(crate) xkb_keycode: u32,
    pub(crate) level: u8,
    pub(crate) added: bool,
}
