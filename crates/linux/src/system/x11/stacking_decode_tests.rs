use super::*;

fn bytes_of(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_ne_bytes())
        .collect()
}

const STATES: StateAtoms = StateAtoms {
    hidden: 301,
    above: 302,
};

const TYPES: TypeAtoms = TypeAtoms {
    normal: 401,
    dialog: 402,
    utility: 403,
    menu: [404, 405, 406],
    dock: 407,
    desktop: 408,
};

#[test]
fn a_stacking_list_keeps_bottom_to_top_order() {
    let decoded = decode_stack(32, &bytes_of(&[0x0040_0003, 0x0060_0001, 7]));
    assert_eq!(decoded, Some(vec![0x0040_0003, 0x0060_0001, 7]));
}

#[test]
fn a_truncated_stacking_list_is_rejected() {
    let mut bytes = bytes_of(&[1, 2, 3]);
    bytes.pop();
    assert_eq!(decode_stack(32, &bytes), None);
}

#[test]
fn hidden_and_above_atoms_decode_to_both_flags() {
    let state = decode_state(32, &bytes_of(&[STATES.hidden, 999, STATES.above]), STATES);
    assert_eq!(state, (true, true));
}

#[test]
fn an_unknown_state_atom_is_ignored() {
    assert_eq!(decode_state(32, &bytes_of(&[999]), STATES), (false, false));
    assert_eq!(
        decode_state(32, &bytes_of(&[STATES.above]), STATES),
        (false, true)
    );
}

#[test]
fn a_state_property_that_cannot_be_decoded_carries_no_flag() {
    assert_eq!(decode_state(8, &[1, 2, 3, 4], STATES), (false, false));
}

#[test]
fn desktop_ffffffff_is_sticky_and_an_index_is_that_desktop() {
    assert_eq!(
        decode_desktop(32, &bytes_of(&[0xFFFF_FFFF])),
        DesktopPlacement::Sticky
    );
    assert_eq!(decode_desktop(32, &bytes_of(&[2])), DesktopPlacement::On(2));
    assert_eq!(decode_desktop(32, &[]), DesktopPlacement::Unknown);
    assert_eq!(decode_desktop(8, &[1]), DesktopPlacement::Unknown);
}

#[test]
fn the_current_desktop_is_the_single_cardinal() {
    assert_eq!(decode_current_desktop(32, &bytes_of(&[1])), Some(1));
    assert_eq!(decode_current_desktop(32, &bytes_of(&[1, 2])), None);
}

#[test]
fn the_first_listed_window_type_names_the_kind() {
    let kind = |atoms: &[u32]| decode_kind(32, &bytes_of(atoms), TYPES);
    assert_eq!(kind(&[TYPES.dialog, TYPES.normal]), WindowKind::Dialog);
    assert_eq!(kind(&[TYPES.menu[2]]), WindowKind::Menu);
    assert_eq!(kind(&[TYPES.dock]), WindowKind::Dock);
    assert_eq!(kind(&[TYPES.desktop]), WindowKind::Desktop);
    assert_eq!(kind(&[TYPES.utility]), WindowKind::Utility);
    assert_eq!(kind(&[999]), WindowKind::Other);
}

#[test]
fn a_window_with_no_type_is_a_normal_window() {
    assert_eq!(decode_kind(32, &[], TYPES), WindowKind::Normal);
}

#[test]
fn frame_extents_expand_a_client_rectangle_into_its_frame() {
    let client = Extents {
        x: 100,
        y: 157,
        width: 320,
        height: 200,
    };
    let border = FrameExtents {
        left: 0,
        right: 0,
        top: 37,
        bottom: 0,
    };
    assert_eq!(
        frame_rectangle(client, Some(border)),
        Extents {
            x: 100,
            y: 120,
            width: 320,
            height: 237
        }
    );
    assert_eq!(frame_rectangle(client, None), client);
}

#[test]
fn every_side_of_the_border_grows_the_matching_edge() {
    let client = Extents {
        x: 10,
        y: 20,
        width: 100,
        height: 50,
    };
    let border = FrameExtents {
        left: 1,
        right: 2,
        top: 3,
        bottom: 4,
    };
    assert_eq!(
        frame_rectangle(client, Some(border)),
        Extents {
            x: 9,
            y: 17,
            width: 103,
            height: 57
        }
    );
}

#[test]
fn a_property_cut_short_by_the_length_asked_for_is_not_a_value() {
    assert_eq!(
        complete_value(32, vec![1, 2, 3, 4], 0),
        Some((32, vec![1, 2, 3, 4]))
    );
    assert_eq!(complete_value(32, vec![1, 2, 3, 4], 8), None);
    assert_eq!(complete_value(0, Vec::new(), 0), Some((0, Vec::new())));
}

#[test]
fn more_windows_than_a_read_probes_exceed_the_cap() {
    assert!(!exceeds_cap(MAX_ENTRIES));
    assert!(exceeds_cap(MAX_ENTRIES + 1));
}
