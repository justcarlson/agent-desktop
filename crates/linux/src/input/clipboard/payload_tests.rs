use super::*;
use agent_desktop_core::{ImageBuffer, ImageFormat};

fn target(name: &str, bytes: &[u8]) -> Target {
    Target {
        name: name.to_owned(),
        bytes: bytes.to_vec(),
    }
}

fn three() -> Payload {
    Payload {
        targets: vec![
            target("a", b"one"),
            target("bb", b""),
            target("ccc", &[0, 255, 7]),
        ],
    }
}

fn tiny_png(width: u32, height: u32) -> Vec<u8> {
    let mut data = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[8, 2, 0, 0, 0]);
    data.extend_from_slice(&[0; 4]);
    data
}

fn image(width: u32, height: u32, data: Vec<u8>) -> ClipboardContent {
    ClipboardContent::Image(ImageBuffer {
        data,
        format: ImageFormat::Png,
        width,
        height,
        scale_factor: 1.0,
    })
}

#[test]
fn framing_round_trips_three_targets() {
    let payload = three();
    assert_eq!(unframe(&frame(&payload)).expect("round trip"), payload);
}

#[test]
fn a_length_that_overruns_the_buffer_is_rejected() {
    let mut bytes = frame(&three());
    bytes.truncate(bytes.len() - 1);
    assert!(unframe(&bytes).is_err());
    let mut longer = frame(&three());
    longer.push(0);
    assert!(unframe(&longer).is_err());
}

#[test]
fn a_zero_target_payload_is_rejected() {
    let bytes = frame(&Payload {
        targets: Vec::new(),
    });
    assert!(unframe(&bytes).is_err());
}

#[test]
fn an_unknown_generation_is_rejected() {
    let mut bytes = frame(&three());
    bytes[0] = GENERATION + 1;
    let error = unframe(&bytes).expect_err("generation");
    assert!(error.message.contains("generation"), "{error:?}");
}

#[test]
fn an_empty_buffer_and_a_huge_count_are_rejected_without_allocating() {
    assert!(unframe(&[]).is_err());
    let mut bytes = vec![GENERATION];
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    assert!(unframe(&bytes).is_err());
}

#[test]
fn text_becomes_the_text_targets() {
    let payload = payload_of(&ClipboardContent::Text("hi".into())).expect("text");
    let names: Vec<&str> = payload.targets.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "UTF8_STRING",
            "text/plain;charset=utf-8",
            "text/plain",
            "STRING"
        ]
    );
}

#[test]
fn file_paths_become_both_list_targets() {
    let payload = payload_of(&ClipboardContent::FileUrls(vec!["/tmp/a b".into()])).expect("paths");
    assert_eq!(payload.targets[0].name, URI_LIST);
    assert_eq!(payload.targets[0].bytes, b"file:///tmp/a%20b\r\n");
    assert_eq!(payload.targets[1].name, GNOME_FILES);
    assert_eq!(payload.targets[1].bytes, b"copy\nfile:///tmp/a%20b");
}

#[test]
fn relative_or_missing_paths_are_invalid_args() {
    for paths in [vec![], vec!["relative.txt".to_owned()]] {
        let error = payload_of(&ClipboardContent::FileUrls(paths)).expect_err("paths");
        assert_eq!(error.code, ErrorCode::InvalidArgs);
    }
}

#[test]
fn an_image_keeps_its_png_bytes_as_the_only_target() {
    let png = tiny_png(3, 2);
    let payload = payload_of(&image(3, 2, png.clone())).expect("image");
    assert_eq!(payload.targets, vec![target(PNG, &png)]);
}

#[test]
fn an_image_whose_metadata_disagrees_with_its_png_is_invalid_args() {
    let error = payload_of(&image(4, 2, tiny_png(3, 2))).expect_err("mismatch");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    let error = payload_of(&image(3, 2, vec![1, 2, 3])).expect_err("not a png");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}

#[test]
fn text_over_the_cap_is_invalid_args() {
    let text = "x".repeat(MAX_PNG_INPUT_BYTES + 1);
    let error = payload_of(&ClipboardContent::Text(text)).expect_err("cap");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}
