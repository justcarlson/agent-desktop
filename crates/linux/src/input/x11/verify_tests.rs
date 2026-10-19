use super::{check, leftover_buttons, leftover_keys};
use crate::input::release_guard::Touched;
use crate::input::x11::fake_sink::FakeSink;
use crate::input::x11::pointer_read::PointerRead;

fn read(mask: u16) -> PointerRead {
    PointerRead { x: 0, y: 0, mask }
}

#[test]
fn a_button_the_call_pressed_and_the_mask_still_shows_is_leftover() {
    assert_eq!(leftover_buttons(&read(0x0100), &[1]), vec![1]);
    assert_eq!(leftover_buttons(&read(0x0400), &[3, 1]), vec![3]);
}

#[test]
fn a_button_the_call_did_not_press_is_ignored() {
    assert!(leftover_buttons(&read(0x0400), &[1]).is_empty());
}

#[test]
fn a_clear_mask_has_no_leftover() {
    assert!(leftover_buttons(&read(0), &[1, 2, 3]).is_empty());
}

#[test]
fn a_key_the_call_pressed_and_the_keymap_still_shows_is_leftover() {
    let mut bits = [0u8; 32];
    bits[4] = 1 << 6;
    assert_eq!(leftover_keys(&bits, &[38, 50]), vec![38]);
}

#[test]
fn a_key_the_call_did_not_press_is_ignored() {
    let mut bits = [0u8; 32];
    bits[4] = 1 << 6;
    assert!(leftover_keys(&bits, &[39]).is_empty());
}

#[test]
fn a_leftover_button_fails_the_call_loudly() {
    let fake = FakeSink::new();
    fake.world().sticky_buttons.push(1);
    let mut sink = fake.clone();
    let touched = Touched {
        keys: Vec::new(),
        buttons: vec![1],
    };

    let error = check(&mut sink, &touched).expect_err("a held button");

    assert_eq!(error.details.expect("details")["leftover_buttons"][0], 1);
}

#[test]
fn nothing_touched_reads_nothing_and_passes() {
    let fake = FakeSink::new();
    fake.world().sticky_buttons.push(1);
    let mut sink = fake.clone();

    assert!(check(&mut sink, &Touched::default()).is_ok());
}
