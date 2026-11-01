use super::*;

const OWNER_US_RU: &str = include_str!("us_ru.xkb");
const KEY_LEFTSHIFT: u32 = 42;
const KEY_LEFTCTRL: u32 = 29;
const KEY_A: u32 = 30;

fn us_ru() -> XkbKeymap {
    XkbKeymap::compile(OWNER_US_RU.as_bytes())
        .expect("libxkbcommon loads")
        .expect("the us,ru keymap compiles")
}

#[test]
fn a_new_state_holds_no_modifier_and_locks_the_requested_layout() {
    let keymap = us_ru();
    for layout in [0, 1] {
        let state = XkbState::new(&keymap, layout).expect("the state builds");
        assert_eq!(
            state.modifiers().expect("the state reads"),
            ModifierState {
                group: layout,
                ..ModifierState::default()
            }
        );
    }
}

#[test]
fn a_held_modifier_is_depressed_until_its_key_goes_up_and_the_layout_stays() {
    let keymap = us_ru();
    let shift = keymap.mod_mask(c"Shift").expect("the Shift mask reads");
    let control = keymap.mod_mask(c"Control").expect("the Control mask reads");
    let mut state = XkbState::new(&keymap, 1).expect("the state builds");
    let held = state
        .update_key(KEY_LEFTSHIFT, true)
        .expect("shift goes down");
    assert_eq!((held.depressed, held.group), (shift, 1));
    let both = state
        .update_key(KEY_LEFTCTRL, true)
        .expect("ctrl goes down");
    assert_eq!(both.depressed, shift | control);
    let typed = state.update_key(KEY_A, true).expect("a goes down");
    assert_eq!(typed.depressed, shift | control);
    state.update_key(KEY_A, false).expect("a goes up");
    state.update_key(KEY_LEFTCTRL, false).expect("ctrl goes up");
    let released = state
        .update_key(KEY_LEFTSHIFT, false)
        .expect("shift goes up");
    assert_eq!(
        released,
        ModifierState {
            group: 1,
            ..ModifierState::default()
        }
    );
}
