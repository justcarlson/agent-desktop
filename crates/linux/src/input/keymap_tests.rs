use super::*;

const A: u32 = 0x61;
const CAP_A: u32 = 0x41;

#[test]
fn a_keysym_on_level_one_resolves_without_shift() {
    assert_eq!(
        Keymap::sample().lookup(A),
        Some(KeyLookup {
            code: 10,
            shift: false
        })
    );
}

#[test]
fn a_keysym_on_level_two_resolves_to_its_keycode_with_shift() {
    assert_eq!(
        Keymap::sample().lookup(CAP_A),
        Some(KeyLookup {
            code: 10,
            shift: true
        })
    );
    assert_eq!(
        Keymap::sample().lookup(0x21),
        Some(KeyLookup {
            code: 11,
            shift: true
        })
    );
}

#[test]
fn an_unshifted_match_wins_over_a_shifted_one_on_an_earlier_keycode() {
    let map = Keymap::parse(8, 2, vec![0x30, 0x41, 0x41, 0x42], 0, Vec::new());
    assert_eq!(
        map.lookup(0x41),
        Some(KeyLookup {
            code: 9,
            shift: false
        })
    );
}

#[test]
fn a_keysym_absent_from_the_layout_or_zero_has_no_lookup() {
    assert_eq!(Keymap::sample().lookup(0x4f60), None);
    assert_eq!(Keymap::sample().lookup(0), None);
}

#[test]
fn group_two_columns_are_not_reachable_without_a_group_switch() {
    let map = Keymap::parse(8, 4, vec![0x61, 0x41, 0x3b1, 0x391], 0, Vec::new());
    assert_eq!(map.lookup(0x3b1), None);
}

#[test]
fn modifier_codes_come_from_the_first_nonzero_entry_of_the_row() {
    let map = Keymap::sample();
    assert_eq!(map.modifier_code(&Modifier::Shift), Some(8));
    assert_eq!(map.modifier_code(&Modifier::Ctrl), Some(9));
    assert_eq!(map.modifier_code(&Modifier::Alt), Some(12));
    assert_eq!(map.modifier_code(&Modifier::Meta), Some(15));
}

#[test]
fn a_mapping_with_no_modifier_table_has_no_modifier_codes() {
    let map = Keymap::parse(8, 2, vec![1, 2], 0, Vec::new());
    assert_eq!(map.modifier_code(&Modifier::Shift), None);
}

#[test]
fn the_spare_keycode_is_the_highest_one_with_no_symbols() {
    assert_eq!(Keymap::sample().spare_keycodes(1), vec![14]);
    assert_eq!(Keymap::sample().spare_keycodes(5), vec![14, 12]);
}

#[test]
fn a_mapping_with_no_empty_keycode_has_no_spare() {
    let map = Keymap::parse(8, 2, vec![1, 2, 3, 4], 0, Vec::new());
    assert!(map.spare_keycodes(1).is_empty());
}

#[test]
fn keysyms_of_returns_every_column_of_a_keycode() {
    assert_eq!(Keymap::sample().keysyms_of(10), Some(vec![A, CAP_A, 0, 0]));
    assert_eq!(Keymap::sample().keysyms_of(200), None);
    assert_eq!(Keymap::sample().keysyms_of(3), None);
}
