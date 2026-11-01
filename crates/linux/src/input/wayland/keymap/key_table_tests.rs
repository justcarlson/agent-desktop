use super::super::evdev_key::EvdevKey;
use super::super::press_state::PressState;
use super::KeyTable;
use crate::input::keyboard::key_names::{KEYSYM_SHIFT_L, KEYSYM_TAB};

const OWNER_DE: &str = include_str!("de_nodeadkeys_compose_ralt.xkb");
const KEYSYM_Z_LOWER: u32 = 0x7a;
const KEYSYM_Z_UPPER: u32 = 0x5a;
const KEYSYM_AT: u32 = 0x40;
const KEYSYM_ISO_LEVEL3_SHIFT: u32 = 0xfe03;
const EVDEV_LEFT_SHIFT: u32 = 42;
const EVDEV_Z_ON_QWERTZ: u32 = 21;

fn de() -> KeyTable {
    let mut text = OWNER_DE.as_bytes().to_vec();
    text.push(0);
    KeyTable::from_keymap(&text, 0).expect("the de keymap reads")
}

#[test]
fn only_level_one_and_shift_level_two_of_the_active_layout_count() {
    let table = de();
    assert_eq!(
        table.lookup(KEYSYM_Z_LOWER),
        Some(EvdevKey {
            code: EVDEV_Z_ON_QWERTZ,
            shift: false
        })
    );
    assert_eq!(
        table.lookup(KEYSYM_Z_UPPER),
        Some(EvdevKey {
            code: EVDEV_Z_ON_QWERTZ,
            shift: true
        })
    );
    assert_eq!(table.lookup(KEYSYM_TAB).map(|key| key.shift), Some(false));
    assert!(table.lookup(KEYSYM_ISO_LEVEL3_SHIFT).is_some());
    assert_eq!(table.lookup(KEYSYM_AT), None, "@ is AltGr+Q on de");
}

#[test]
fn a_shift_level_key_is_wrapped_in_its_own_shift() {
    let table = de();
    let mut state = PressState::default();
    assert_eq!(
        table.plan(KEYSYM_Z_UPPER, true, &mut state),
        Some(vec![(EVDEV_LEFT_SHIFT, true), (EVDEV_Z_ON_QWERTZ, true)])
    );
    assert_eq!(state.wrapped, [EVDEV_Z_ON_QWERTZ]);
    assert_eq!(
        table.plan(KEYSYM_Z_UPPER, false, &mut state),
        Some(vec![(EVDEV_Z_ON_QWERTZ, false), (EVDEV_LEFT_SHIFT, false)])
    );
    assert_eq!(state, PressState::default());
}

#[test]
fn a_held_shift_is_not_wrapped_again() {
    let table = de();
    let mut state = PressState::default();
    assert_eq!(
        table.plan(KEYSYM_SHIFT_L, true, &mut state),
        Some(vec![(EVDEV_LEFT_SHIFT, true)])
    );
    assert_eq!(
        table.plan(KEYSYM_Z_UPPER, true, &mut state),
        Some(vec![(EVDEV_Z_ON_QWERTZ, true)])
    );
    assert_eq!(
        table.plan(KEYSYM_Z_UPPER, false, &mut state),
        Some(vec![(EVDEV_Z_ON_QWERTZ, false)])
    );
    assert_eq!(
        table.plan(KEYSYM_SHIFT_L, false, &mut state),
        Some(vec![(EVDEV_LEFT_SHIFT, false)])
    );
    assert_eq!(state, PressState::default());
}

#[test]
fn a_keysym_off_the_table_has_no_plan() {
    let mut state = PressState::default();
    assert_eq!(de().plan(KEYSYM_AT, true, &mut state), None);
    assert_eq!(state, PressState::default());
}
