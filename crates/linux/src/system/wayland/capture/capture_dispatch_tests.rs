use std::time::Duration;

use super::*;

#[test]
fn a_timestamp_joins_both_halves_of_its_seconds() {
    assert_eq!(stamp(0, 6, 169_297_839), Duration::new(6, 169_297_839));
    assert_eq!(stamp(1, 0, 0), Duration::from_secs(1 << 32));
}

#[test]
fn an_out_of_range_nanosecond_field_stays_within_its_second() {
    assert_eq!(stamp(0, 6, 2_000_000_000), Duration::new(6, 999_999_999));
}
