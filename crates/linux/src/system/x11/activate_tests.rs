use super::*;

#[test]
fn the_request_names_source_and_timestamp_in_the_first_two_longs() {
    assert_eq!(activation_data(2, 123_456), [2, 123_456, 0, 0, 0]);
}

#[test]
fn the_message_targets_the_client_with_the_active_window_atom() {
    let event = activation_event(0x0040_0003, 77, ACTIVATION_SOURCE, 5_000);
    assert_eq!(event.window, 0x0040_0003);
    assert_eq!(event.type_, 77);
    assert_eq!(event.format, 32);
    assert_eq!(event.data.as_data32(), [2, 5_000, 0, 0, 0]);
}
