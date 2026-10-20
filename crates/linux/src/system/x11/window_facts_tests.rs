use super::*;

const WANTED: [u32; 3] = [10, 20, 30];

fn bytes(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_ne_bytes())
        .collect()
}

#[test]
fn three_atoms_with_hidden_among_them_decode_hidden() {
    let decoded = decode_state(32, &bytes(&[99, 10, 98]), 0, WANTED);
    assert_eq!(decoded, Some([true, false, false]));
}

#[test]
fn both_maximized_atoms_decode_independently() {
    assert_eq!(
        decode_state(32, &bytes(&[20, 30]), 0, WANTED),
        Some([false, true, true])
    );
    assert_eq!(
        decode_state(32, &bytes(&[30]), 0, WANTED),
        Some([false, false, true])
    );
}

#[test]
fn an_empty_property_decodes_all_false() {
    assert_eq!(decode_state(0, &[], 0, WANTED), Some([false; 3]));
}

#[test]
fn a_property_of_the_wrong_format_is_unknown_not_false() {
    assert_eq!(decode_state(8, &bytes(&[10]), 0, WANTED), None);
}

#[test]
fn a_property_that_ends_inside_an_atom_is_unknown_not_false() {
    let mut cut = bytes(&[10, 20]);
    cut.pop();
    assert_eq!(decode_state(32, &cut, 0, WANTED), None);
}

#[test]
fn a_property_cut_short_by_the_length_asked_for_is_unknown_not_false() {
    assert_eq!(decode_state(32, &bytes(&[99]), 8, WANTED), None);
}
