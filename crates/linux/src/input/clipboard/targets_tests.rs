use super::*;

fn offered(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut data = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    data.extend_from_slice(b"IHDR");
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    data
}

#[test]
fn auto_prefers_file_paths_over_text() {
    let list = offered(&["TARGETS", "UTF8_STRING", "text/uri-list"]);
    assert_eq!(
        choose_target(&list, ClipboardFormat::Auto),
        Some(Wanted::FileUrls(URI_LIST))
    );
}

#[test]
fn auto_prefers_an_image_over_text() {
    let list = offered(&["UTF8_STRING", "image/png"]);
    assert_eq!(
        choose_target(&list, ClipboardFormat::Auto),
        Some(Wanted::Image)
    );
}

#[test]
fn auto_with_only_text_picks_the_best_text_target() {
    let list = offered(&["STRING", "text/plain", "TARGETS"]);
    assert_eq!(
        choose_target(&list, ClipboardFormat::Auto),
        Some(Wanted::Text("text/plain"))
    );
    let both = offered(&["STRING", "UTF8_STRING"]);
    assert_eq!(
        choose_target(&both, ClipboardFormat::Text),
        Some(Wanted::Text("UTF8_STRING"))
    );
}

#[test]
fn only_bookkeeping_targets_pick_nothing() {
    let list = offered(&["TARGETS", "TIMESTAMP", "MULTIPLE"]);
    assert_eq!(choose_target(&list, ClipboardFormat::Auto), None);
}

#[test]
fn text_requested_with_only_an_image_offered_is_absent() {
    let list = offered(&["image/png"]);
    assert_eq!(choose_target(&list, ClipboardFormat::Text), None);
}

#[test]
fn file_paths_fall_back_to_the_gnome_target() {
    let list = offered(&["x-special/gnome-copied-files"]);
    assert_eq!(
        choose_target(&list, ClipboardFormat::FileUrls),
        Some(Wanted::FileUrls(GNOME_FILES))
    );
}

#[test]
fn an_image_other_than_png_is_not_offered_as_an_image() {
    let list = offered(&["image/bmp", "image/jpeg"]);
    assert_eq!(choose_target(&list, ClipboardFormat::Image), None);
}

#[test]
fn an_image_reads_with_its_png_dimensions_and_scale_one() {
    let content = content_of(&Wanted::Image, png(7, 5)).expect("png");
    let ClipboardContent::Image(image) = content else {
        panic!("expected an image")
    };
    assert_eq!((image.width, image.height, image.scale_factor), (7, 5, 1.0));
}

#[test]
fn bytes_that_are_not_a_png_are_action_failed() {
    let error = content_of(&Wanted::Image, vec![1, 2, 3]).expect_err("not png");
    assert_eq!(error.code, ErrorCode::ActionFailed);
}

#[test]
fn text_and_file_lists_decode_through_their_targets() {
    let text = content_of(&Wanted::Text("STRING"), vec![0xe9]).expect("text");
    assert!(matches!(text, ClipboardContent::Text(ref s) if s == "é"));
    let files =
        content_of(&Wanted::FileUrls(GNOME_FILES), b"cut\nfile:///a".to_vec()).expect("files");
    assert!(matches!(files, ClipboardContent::FileUrls(ref p) if p == &["/a".to_owned()]));
}

#[test]
fn bytes_over_the_cap_are_refused() {
    let error = content_of(
        &Wanted::Text("UTF8_STRING"),
        vec![b'x'; MAX_PNG_INPUT_BYTES + 1],
    )
    .expect_err("cap");
    assert_eq!(error.code, ErrorCode::ActionFailed);
}
