use super::*;

fn lookup(map: &Keymap, keysym: u32) -> Option<(u8, bool)> {
    map.lookup(keysym).map(|entry| (entry.code, entry.shift))
}

#[test]
fn lookup_layout_boundaries() {
    let map = Keymap::sample();
    assert_eq!(lookup(&map, 0x61), Some((10, false)));
    assert_eq!(lookup(&map, 0x41), Some((10, true)));
    assert_eq!(lookup(&map, 0x21), Some((11, true)));
    assert_eq!(lookup(&map, 0), None);
    assert_eq!(lookup(&map, 0x4f60), None);
    assert_eq!(
        lookup(
            &Keymap::new(8, 2, vec![0x30, 0x41, 0x41, 0x42], 0, vec![]),
            0x41
        ),
        Some((9, false))
    );
    assert_eq!(
        lookup(
            &Keymap::new(8, 4, vec![0x61, 0x41, 0x3b1, 0x391], 0, vec![]),
            0x3b1
        ),
        None
    );
    let ragged = Keymap::new(8, 2, vec![0x61, 0x41, 0x62], 0, vec![]);
    assert_eq!(lookup(&ragged, 0x62), None);
    assert_eq!(ragged.keysyms_of(9), None);
    assert_eq!(Keymap::new(8, 0, vec![1], 0, vec![]).keysyms_of(8), None);
    let modifiers = Keymap::new(
        8,
        1,
        vec![1],
        2,
        vec![0, 8, 0, 9, 0, 12, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0],
    );
    for (modifier, code) in [
        (Modifier::Shift, 8),
        (Modifier::Ctrl, 9),
        (Modifier::Alt, 12),
        (Modifier::Meta, 15),
    ] {
        assert_eq!(modifiers.modifier_code(&modifier), Some(code));
    }
}

#[test]
fn spare_rows_are_bounded() {
    let map = Keymap::sample();
    assert_eq!(map.spare_keycodes(1), [14]);
    assert_eq!(map.spare_keycodes(5), [14, 12]);
    assert_eq!(map.keysyms_of(14), Some(vec![0; 4]));
    let full = Keymap::new(8, 2, vec![1, 2, 3, 4], 0, vec![]);
    assert!(full.spare_keycodes(1).is_empty());
    assert_eq!(full.keysyms_of(9), Some(vec![3, 4]));
    assert_eq!(full.keysyms_of(200), None);
    assert_eq!(full.keysyms_of(3), None);
}
