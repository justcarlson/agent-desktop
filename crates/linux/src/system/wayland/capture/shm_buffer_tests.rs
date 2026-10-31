use super::*;

fn layout(width: u32, height: u32, stride: u32) -> BufferLayout {
    BufferLayout {
        format: 1,
        width,
        height,
        stride,
    }
}

#[test]
fn a_buffer_holds_every_row_at_its_stride() {
    assert_eq!(byte_len(layout(1280, 800, 5120)), Some(5120 * 800));
    assert_eq!(byte_len(layout(3, 2, 16)), Some(32));
}

#[test]
fn an_empty_short_strided_or_oversized_layout_is_refused() {
    assert_eq!(byte_len(layout(0, 800, 5120)), None);
    assert_eq!(byte_len(layout(1280, 0, 5120)), None);
    assert_eq!(byte_len(layout(1280, 800, 5119)), None);
    assert_eq!(byte_len(layout(9000, 9000, 36000)), None);
    assert_eq!(byte_len(layout(u32::MAX, 1, u32::MAX)), None);
}
