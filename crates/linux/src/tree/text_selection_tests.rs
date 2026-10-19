use std::ops::Range;

use agent_desktop_core::{AdapterError, ErrorCode};

use super::{MAX_TEXT_CHARACTERS, TEXT_INTERFACE, TextSource, readable, selection_utf16};

const MIXED: &str = "a\u{1F600}e\u{301}b";

#[derive(Default)]
struct Fake {
    caret: Option<usize>,
    selections: Option<Vec<Range<usize>>>,
    count: Option<usize>,
    text: Option<String>,
    text_reads: usize,
    gone: bool,
}

impl Fake {
    fn over(text: &str, caret: usize, selections: Vec<Range<usize>>) -> Self {
        Self {
            caret: Some(caret),
            selections: Some(selections),
            count: Some(text.chars().count()),
            text: Some(text.to_owned()),
            ..Self::default()
        }
    }
}

impl TextSource for Fake {
    fn caret(&mut self) -> Result<Option<usize>, AdapterError> {
        if self.gone {
            return Err(AdapterError::new(ErrorCode::AppNotFound, "gone"));
        }
        Ok(self.caret)
    }

    fn selections(&mut self) -> Result<Option<Vec<Range<usize>>>, AdapterError> {
        Ok(self.selections.clone())
    }

    fn character_count(&mut self) -> Result<Option<usize>, AdapterError> {
        Ok(self.count)
    }

    fn text(&mut self, _end: usize) -> Result<Option<String>, AdapterError> {
        self.text_reads += 1;
        Ok(self.text.clone())
    }
}

fn one(start: usize, end: usize) -> Vec<Range<usize>> {
    std::iter::once(Range { start, end }).collect()
}

fn selection(fake: &mut Fake) -> Option<Range<usize>> {
    selection_utf16(fake).expect("the source answers")
}

#[test]
fn a_caret_with_no_selection_is_an_empty_range_in_utf16_units() {
    let mut fake = Fake::over(MIXED, 3, Vec::new());
    assert_eq!(selection(&mut fake), Some(4..4));
}

#[test]
fn a_selection_converts_from_code_points_to_utf16_units() {
    let mut fake = Fake::over(MIXED, 3, one(1, 2));
    assert_eq!(selection(&mut fake), Some(1..3));
}

#[test]
fn a_backwards_selection_is_put_in_order() {
    let mut fake = Fake::over(MIXED, 3, one(2, 1));
    assert_eq!(selection(&mut fake), Some(1..3));
}

#[test]
fn a_failed_caret_read_is_unavailable() {
    let mut fake = Fake::over(MIXED, 3, Vec::new());
    fake.caret = None;
    assert_eq!(selection(&mut fake), None);
}

#[test]
fn a_failed_selection_read_is_unavailable() {
    let mut fake = Fake::over(MIXED, 3, Vec::new());
    fake.selections = None;
    assert_eq!(selection(&mut fake), None);
}

#[test]
fn a_text_longer_than_the_cap_is_unavailable_without_a_text_read() {
    let mut fake = Fake::over(MIXED, 3, Vec::new());
    fake.count = Some(MAX_TEXT_CHARACTERS + 1);
    assert_eq!(selection(&mut fake), None);
    assert_eq!(fake.text_reads, 0);
}

#[test]
fn a_text_of_exactly_the_cap_is_read() {
    let mut fake = Fake::over(MIXED, 3, Vec::new());
    fake.count = Some(MAX_TEXT_CHARACTERS);
    assert_eq!(selection(&mut fake), Some(4..4));
}

#[test]
fn an_offset_past_the_text_is_unavailable() {
    let mut fake = Fake::over(MIXED, 9, Vec::new());
    assert_eq!(selection(&mut fake), None);
}

#[test]
fn a_gone_node_is_an_error_not_unavailable() {
    let mut fake = Fake::over(MIXED, 3, Vec::new());
    fake.gone = true;
    let error = selection_utf16(&mut fake).expect_err("a gone node is refused");
    assert_eq!(error.code, ErrorCode::AppNotFound);
}

fn interfaces(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_secure_field_is_never_read() {
    assert!(!readable(true, Some(&interfaces(&[TEXT_INTERFACE]))));
}

#[test]
fn only_a_node_naming_the_text_interface_is_read() {
    assert!(readable(false, Some(&interfaces(&[TEXT_INTERFACE]))));
    assert!(!readable(
        false,
        Some(&interfaces(&["org.a11y.atspi.Value"]))
    ));
    assert!(!readable(false, None));
}
