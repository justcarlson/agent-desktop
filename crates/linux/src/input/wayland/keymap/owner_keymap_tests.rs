use agent_desktop_core::DeliverySemantics;

use super::super::xkb_keymap::XkbKeymap;
use super::{ExtendedKeymap, KeyPlacement, extend};
use crate::input::keyboard::key_names::KEYSYM_TAB;

const OWNER_DE: &str = include_str!("de_nodeadkeys_compose_ralt.xkb");
const OWNER_US_RU: &str = include_str!("us_ru.xkb");
const KEYSYM_A_LOWER: u32 = 0x61;
const KEYSYM_Z_LOWER: u32 = 0x7a;
const KEYSYM_Z_UPPER: u32 = 0x5a;
const KEYSYM_AT: u32 = 0x40;
const KEYSYM_EACUTE: u32 = 0xe9;
const KEYSYM_CHECK_MARK: u32 = 0x100_2713;
const KEYSYM_CYRILLIC_A_LOWER: u32 = 0x6c1;
const KEYSYM_CYRILLIC_A_UPPER: u32 = 0x6e1;
const KEYSYM_UNICODE_CYRILLIC_A: u32 = 0x100_0430;
const OWNER_DE_UNBOUND_KEYCODES: [u32; 15] = [
    8, 97, 103, 120, 132, 149, 154, 168, 178, 183, 184, 219, 222, 230, 248,
];
const LAYOUT_US: u32 = 0;
const LAYOUT_RU: u32 = 1;

fn placement(placements: &[KeyPlacement], keysym: u32) -> &KeyPlacement {
    placements
        .iter()
        .find(|placement| placement.keysym == keysym)
        .unwrap_or_else(|| panic!("no placement for keysym {keysym:#x}"))
}

fn at(placements: &[KeyPlacement], keysym: u32) -> (u32, u8, bool) {
    let found = placement(placements, keysym);
    (found.xkb_keycode, found.level, found.added)
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

fn assert_round_trip(extended: &ExtendedKeymap, layout: u32) {
    let compiled = XkbKeymap::compile(extended.text.as_bytes())
        .expect("libxkbcommon loads")
        .expect("the extended keymap compiles");
    let shift = compiled.mod_mask(c"Shift").expect("the Shift mask reads");
    for placement in &extended.placements {
        let held = if placement.level == 2 { shift } else { 0 };
        let typed = compiled
            .typed_keysym(placement.xkb_keycode, layout, held)
            .expect("the keysym resolves");
        let same_character = compiled.keysym_char(typed).expect("utf32 reads")
            == compiled.keysym_char(placement.keysym).expect("utf32 reads")
            && compiled.keysym_char(typed).expect("utf32 reads").is_some();
        assert!(
            typed == placement.keysym || same_character,
            "keycode {} level {} types {typed:#x}, not {:#x}",
            placement.xkb_keycode,
            placement.level,
            placement.keysym
        );
    }
}

fn refused_cause(owner: &str, keysyms: &[u32]) -> String {
    let error = extend(owner, keysyms, LAYOUT_US).expect_err("the extension refuses");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.expect("refusal details");
    assert_eq!(details["physical_delivery_started"], false);
    details["cause"].as_str().unwrap_or_default().to_owned()
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
    let extended = extend(OWNER_DE, &requested, LAYOUT_US).expect("the de keymap extends");

    assert_eq!(at(&extended.placements, KEYSYM_TAB), (23, 1, false));
    assert_eq!(at(&extended.placements, KEYSYM_Z_LOWER), (29, 1, false));
    assert_eq!(at(&extended.placements, KEYSYM_Z_UPPER), (29, 2, false));

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
    assert_round_trip(&extended, LAYOUT_US);
}

#[test]
fn an_altgr_only_keysym_counts_as_missing() {
    let extended = extend(OWNER_DE, &[KEYSYM_AT], LAYOUT_US).expect("the de keymap extends");
    let at_sign = placement(&extended.placements, KEYSYM_AT);
    assert!(at_sign.added, "@ is AltGr+Q on de and must get its own key");
    assert_eq!(at_sign.level, 1);
    assert_round_trip(&extended, LAYOUT_US);
}

#[test]
fn an_unnamed_free_keycode_gets_a_name_line() {
    let owner = OWNER_DE.replace("\t<I120>               = 120;\n", "");
    assert_ne!(owner, OWNER_DE, "the fixture names keycode 120");
    let wanted = [0x100_2600, 0x100_2601, 0x100_2602];
    let extended = extend(&owner, &wanted, LAYOUT_US).expect("three keys fit");
    let extra = lines_beyond_owner(&owner, &extended.text);
    let names: Vec<&&str> = extra
        .iter()
        .filter(|line| line.trim_start().starts_with('<'))
        .collect();
    assert_eq!(names, [&"\t<I120>               = 120;"], "{extra:?}");
    assert_eq!(at(&extended.placements, 0x100_2602), (120, 1, true));
    assert_round_trip(&extended, LAYOUT_US);
}

#[test]
fn placements_follow_the_active_layout() {
    let requested = [
        KEYSYM_TAB,
        KEYSYM_A_LOWER,
        KEYSYM_CYRILLIC_A_LOWER,
        KEYSYM_CYRILLIC_A_UPPER,
        KEYSYM_UNICODE_CYRILLIC_A,
    ];

    let us = extend(OWNER_US_RU, &requested, LAYOUT_US).expect("us,ru extends under us");
    assert_eq!(at(&us.placements, KEYSYM_TAB), (23, 1, false));
    assert_eq!(at(&us.placements, KEYSYM_A_LOWER), (38, 1, false));
    for cyrillic in [
        KEYSYM_CYRILLIC_A_LOWER,
        KEYSYM_CYRILLIC_A_UPPER,
        KEYSYM_UNICODE_CYRILLIC_A,
    ] {
        let added = placement(&us.placements, cyrillic);
        assert!(added.added, "{cyrillic:#x} is not on the us layout");
        assert_eq!(added.level, 1);
    }
    assert_round_trip(&us, LAYOUT_US);

    let ru = extend(OWNER_US_RU, &requested, LAYOUT_RU).expect("us,ru extends under ru");
    assert_eq!(at(&ru.placements, KEYSYM_TAB), (23, 1, false));
    assert_eq!(at(&ru.placements, KEYSYM_CYRILLIC_A_LOWER), (41, 1, false));
    assert_eq!(at(&ru.placements, KEYSYM_CYRILLIC_A_UPPER), (41, 2, false));
    assert_eq!(
        at(&ru.placements, KEYSYM_UNICODE_CYRILLIC_A),
        (41, 1, false)
    );
    let latin = placement(&ru.placements, KEYSYM_A_LOWER);
    assert!(latin.added, "a is not on the ru layout");
    assert_round_trip(&ru, LAYOUT_RU);
    lines_beyond_owner(OWNER_US_RU, &ru.text);
}

#[test]
fn too_few_free_keycodes_refuses_before_any_key() {
    let wanted: Vec<u32> = (0..300).map(|offset| 0x100_2600 + offset).collect();
    assert_eq!(refused_cause(OWNER_DE, &wanted), "no_free_keycode");
}

#[test]
fn an_owner_keymap_that_does_not_compile_is_refused() {
    assert_eq!(
        refused_cause("xkb_keymap { this is not a keymap", &[KEYSYM_TAB]),
        "owner_keymap_invalid"
    );
}

#[test]
fn a_keysym_xkb_cannot_name_is_refused() {
    assert_eq!(refused_cause(OWNER_DE, &[0x2000_0000]), "keysym_invalid");
}

#[test]
fn a_trailing_nul_from_the_compositor_is_not_owner_text() {
    let mut owner = OWNER_DE.to_owned();
    owner.push('\0');
    let extended = extend(&owner, &[KEYSYM_EACUTE], LAYOUT_US).expect("the de keymap extends");
    assert!(!extended.text.contains('\0'));
    lines_beyond_owner(OWNER_DE, &extended.text);
}
