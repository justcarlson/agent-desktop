use super::is_blank;

#[test]
fn only_all_zero_bytes_are_blank() {
    assert!(is_blank(&[0, 0, 0, 0]));
    assert!(is_blank(&[]));
    assert!(!is_blank(&[0, 0, 0, 1]));
    assert!(!is_blank(&[0x99, 0x66, 0x33, 0]));
}
