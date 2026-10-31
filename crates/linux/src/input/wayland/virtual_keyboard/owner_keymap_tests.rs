use super::{KeyPlacement, extend};
use crate::input::keyboard::key_names::KEYSYM_TAB;

const OWNER_DE: &str = include_str!("de_nodeadkeys_compose_ralt.xkb");
const KEYSYM_Z_LOWER: u32 = 0x7a;
const KEYSYM_Z_UPPER: u32 = 0x5a;
const KEYSYM_EACUTE: u32 = 0xe9;
const KEYSYM_CHECK_MARK: u32 = 0x100_2713;
const OWNER_DE_UNBOUND_KEYCODES: [u32; 15] = [
    8, 97, 103, 120, 132, 149, 154, 168, 178, 183, 184, 219, 222, 230, 248,
];

fn placement(placements: &[KeyPlacement], keysym: u32) -> &KeyPlacement {
    placements
        .iter()
        .find(|placement| placement.keysym == keysym)
        .unwrap_or_else(|| panic!("no placement for keysym {keysym:#x}"))
}

fn lines_beyond_owner<'a>(owner: &str, extended: &'a str) -> Vec<&'a str> {
    let mut owner_lines = owner.lines().peekable();
    let mut extra = Vec::new();
    for line in extended.lines() {
        if owner_lines.peek() == Some(&line) {
            owner_lines.next();
        } else {
            extra.push(line);
        }
    }
    assert!(
        owner_lines.next().is_none(),
        "the extended keymap dropped or rewrote an owner line"
    );
    extra
}

#[test]
fn owner_keymap_keeps_its_layout_and_adds_only_missing_keysyms() {
    let requested = [
        KEYSYM_TAB,
        KEYSYM_Z_LOWER,
        KEYSYM_Z_UPPER,
        KEYSYM_EACUTE,
        KEYSYM_CHECK_MARK,
    ];
    let extended = extend(OWNER_DE, &requested).expect("the de keymap extends");

    let tab = placement(&extended.placements, KEYSYM_TAB);
    assert_eq!((tab.xkb_keycode, tab.level, tab.added), (23, 1, false));
    let lower = placement(&extended.placements, KEYSYM_Z_LOWER);
    assert_eq!(
        (lower.xkb_keycode, lower.level, lower.added),
        (29, 1, false)
    );
    let upper = placement(&extended.placements, KEYSYM_Z_UPPER);
    assert_eq!(
        (upper.xkb_keycode, upper.level, upper.added),
        (29, 2, false)
    );

    let eacute = placement(&extended.placements, KEYSYM_EACUTE);
    let check = placement(&extended.placements, KEYSYM_CHECK_MARK);
    for added in [eacute, check] {
        assert!(added.added);
        assert_eq!(added.level, 1);
        assert!(OWNER_DE_UNBOUND_KEYCODES.contains(&added.xkb_keycode));
    }
    assert_ne!(eacute.xkb_keycode, check.xkb_keycode);
    assert_eq!(extended.placements.len(), requested.len());

    let extra = lines_beyond_owner(OWNER_DE, &extended.text);
    let added_keys: Vec<&&str> = extra
        .iter()
        .filter(|line| line.trim_start().starts_with("key <"))
        .collect();
    assert_eq!(added_keys.len(), 2, "added key lines: {added_keys:?}");
    let one_level = extra
        .iter()
        .filter(|line| line.contains("type= \"ONE_LEVEL\""))
        .count();
    assert_eq!(one_level, 2, "each added key is ONE_LEVEL: {extra:?}");
    assert!(extra.iter().any(|line| line.contains("eacute")));
    assert!(extra.iter().any(|line| line.contains("U2713")));
}
