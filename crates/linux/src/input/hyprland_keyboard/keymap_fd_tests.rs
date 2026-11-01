use std::fs::File;
use std::io::Write;
use std::os::fd::OwnedFd;

use agent_desktop_core::DeliverySemantics;
use rustix::fs::{MemfdFlags, memfd_create};

use super::{keymap_memfd, read_keymap};

fn holding(bytes: &[u8]) -> OwnedFd {
    let fd = memfd_create("keymap-test", MemfdFlags::CLOEXEC).expect("a memfd opens");
    let mut file = File::from(fd);
    file.write_all(bytes).expect("the memfd takes the bytes");
    OwnedFd::from(file)
}

fn cause(error: &agent_desktop_core::AdapterError) -> String {
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    error.details.as_ref().expect("details")["cause"]
        .as_str()
        .expect("a cause")
        .to_owned()
}

#[test]
fn a_written_keymap_reads_back_whole_with_its_nul_counted() {
    let text = "xkb_keymap { };";
    let (fd, size) = keymap_memfd(text).expect("the memfd is written");
    assert_eq!(size as usize, text.len() + 1);
    assert_eq!(read_keymap(fd, size).expect("reads"), text);
}

#[test]
fn reading_stops_at_the_nul() {
    let read = read_keymap(holding(b"abc\0junk"), 8).expect("reads");
    assert_eq!(read, "abc");
}

#[test]
fn a_short_empty_huge_or_non_text_keymap_is_refused() {
    for (fd, size) in [
        (holding(b"abc"), 10),
        (holding(b""), 0),
        (holding(b"abc"), u32::MAX),
        (holding(&[0xff, 0xfe, 0]), 3),
    ] {
        let error = read_keymap(fd, size).expect_err("refused");
        assert_eq!(cause(&error), "owner_keymap_unreadable");
    }
}

#[test]
fn a_keymap_another_reader_left_at_its_end_still_reads_whole() {
    let fd = holding(b"xkb_keymap { };\0");
    let mut drained = Vec::new();
    std::io::Read::read_to_end(&mut File::from(fd.try_clone().expect("dup")), &mut drained)
        .expect("a reader drains the shared offset");
    assert_eq!(read_keymap(fd, 16).expect("reads"), "xkb_keymap { };");
}
