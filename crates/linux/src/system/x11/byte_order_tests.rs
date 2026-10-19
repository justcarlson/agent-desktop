use x11rb::protocol::xproto::ImageOrder;

use super::msb_first;

#[test]
fn only_the_most_significant_first_order_reads_big_endian() {
    assert!(msb_first(ImageOrder::MSB_FIRST));
    assert!(!msb_first(ImageOrder::LSB_FIRST));
}
