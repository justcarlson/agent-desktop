use super::*;
use crate::system::wayland::capture::shm_layout::{ARGB8888, XRGB8888};

fn offer(format: u32) -> BufferLayout {
    BufferLayout {
        format,
        width: 1280,
        height: 800,
        stride: 5120,
    }
}

#[test]
fn the_opaque_offer_is_chosen_over_one_with_alpha() {
    let chosen = offered(&[offer(ARGB8888), offer(XRGB8888)]);
    assert_eq!(chosen.map(|layout| layout.format), Some(XRGB8888));
}

#[test]
fn an_offer_list_without_a_readable_format_offers_nothing() {
    assert_eq!(offered(&[offer(0x3231_4752)]), None);
    assert_eq!(offered(&[]), None);
}
