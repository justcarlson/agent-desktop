use super::{TextUnitError, code_point_to_utf16, length_as_i32, utf8_len, utf16_to_code_point};

const MIXED: &str = "a\u{1F600}e\u{301}b";

#[test]
fn a_code_point_offset_converts_to_a_utf16_offset() {
    assert_eq!(code_point_to_utf16(MIXED, 0), Some(0));
    assert_eq!(code_point_to_utf16(MIXED, 1), Some(1));
    assert_eq!(code_point_to_utf16(MIXED, 2), Some(3));
    assert_eq!(code_point_to_utf16(MIXED, 4), Some(5));
    assert_eq!(code_point_to_utf16(MIXED, 5), Some(6));
}

#[test]
fn a_code_point_offset_past_the_end_is_rejected() {
    assert_eq!(code_point_to_utf16(MIXED, 6), None);
}

#[test]
fn a_utf16_offset_converts_to_a_code_point_offset() {
    assert_eq!(utf16_to_code_point(MIXED, 0), Some(0));
    assert_eq!(utf16_to_code_point(MIXED, 1), Some(1));
    assert_eq!(utf16_to_code_point(MIXED, 3), Some(2));
    assert_eq!(utf16_to_code_point(MIXED, 6), Some(5));
}

#[test]
fn an_offset_between_a_surrogate_pair_is_rejected() {
    assert_eq!(utf16_to_code_point(MIXED, 2), None);
}

#[test]
fn a_utf16_offset_past_the_end_is_rejected() {
    assert_eq!(utf16_to_code_point(MIXED, 7), None);
}

#[test]
fn the_conversions_round_trip_at_every_code_point() {
    for points in 0..=5 {
        let units = code_point_to_utf16(MIXED, points).expect("in range");
        assert_eq!(utf16_to_code_point(MIXED, units), Some(points));
    }
}

#[test]
fn the_utf8_length_counts_bytes_not_points() {
    assert_eq!(utf8_len("\u{1F600}"), Ok(4));
    assert_eq!(utf8_len(MIXED), Ok(9));
    assert_eq!(utf8_len(""), Ok(0));
}

#[test]
fn a_length_above_i32_max_is_rejected() {
    let limit = i32::MAX as usize;
    assert_eq!(length_as_i32(limit), Ok(i32::MAX));
    assert_eq!(length_as_i32(limit + 1), Err(TextUnitError::TooLong));
}
