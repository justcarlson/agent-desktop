use std::ops::Range;

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};

use super::fakes::{Fake, Script, drive, span};
use super::*;

fn invalid() -> AdapterError {
    AdapterError::new(ErrorCode::InvalidArgs, "bad range")
        .with_disposition(DeliverySemantics::not_delivered())
}

fn replace(fake: &Fake, text: &str) -> Result<(&'static str, DeliveryOutcome), AdapterError> {
    drive(&mut replace_rungs(fake, text, false))
}

fn typed(fake: &Fake, text: &str) -> Result<(&'static str, DeliveryOutcome), AdapterError> {
    drive(&mut insert_rungs(fake, text, false))
}

#[test]
fn a_set_that_reads_back_the_text_is_matched_on_the_first_rung() {
    let fake = Fake::holding("old");
    let (label, outcome) = replace(&fake, "hello").expect("delivered");
    assert_eq!(label, REPLACE_LABEL);
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Matched));
    assert_eq!(fake.logged(), ["set(hello)"]);
}

#[test]
fn a_true_set_with_the_wrong_read_back_is_terminal_and_never_tries_rung_two() {
    let mut fake = Fake::holding("old");
    fake.landing = Some("hell".to_owned());
    let (_, outcome) = replace(&fake, "hello").expect("delivered");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Unchanged));
    assert_eq!(fake.logged(), ["set(hello)"]);
}

#[test]
fn an_absent_set_falls_to_delete_then_insert() {
    let mut fake = Fake::holding("old");
    fake.set = Script::Absent;
    let (label, outcome) = replace(&fake, "hi").expect("delivered");
    assert_eq!(label, REWRITE_LABEL);
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Matched));
    assert_eq!(fake.logged(), ["set(hi)", "delete(0,3)", "insert(0,2,hi)"]);
}

#[test]
fn a_delete_that_lands_before_a_failed_insert_is_delivered_unverified() {
    let mut fake = Fake::holding("old");
    fake.set = Script::Absent;
    fake.insert = Script::Fail(invalid());
    let error = replace(&fake, "hi").expect_err("the insert failed");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert!(error.suggestion.expect("a hint").contains("Read the field"));
}

#[test]
fn an_insert_that_answers_false_after_a_landed_delete_is_delivered_unverified() {
    let mut fake = Fake::holding("old");
    fake.set = Script::Absent;
    fake.insert = Script::Absent;
    let error = replace(&fake, "hi").expect_err("the insert was refused");
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn a_transport_failure_after_a_landed_delete_stays_uncertain() {
    let mut fake = Fake::holding("old");
    fake.set = Script::Absent;
    fake.insert = Script::Fail(
        AdapterError::new(ErrorCode::Timeout, "no reply")
            .with_disposition(DeliverySemantics::uncertain()),
    );
    let error = replace(&fake, "hi").expect_err("uncertain");
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
}

#[test]
fn a_failed_first_delete_is_not_a_partial_write() {
    let mut fake = Fake::holding("old");
    fake.set = Script::Absent;
    fake.delete = Script::Fail(invalid());
    let error = replace(&fake, "hi").expect_err("the delete failed");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn clear_with_an_empty_read_back_is_matched() {
    let fake = Fake::holding("old");
    let (_, outcome) = replace(&fake, "").expect("delivered");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Matched));
}

#[test]
fn clear_with_a_non_empty_read_back_is_not_matched() {
    let mut fake = Fake::holding("old");
    fake.landing = Some("old".to_owned());
    let (_, outcome) = replace(&fake, "").expect("delivered");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Unchanged));
}

#[test]
fn clear_by_deleting_issues_no_insert() {
    let mut fake = Fake::holding("old");
    fake.set = Script::Absent;
    let (label, outcome) = replace(&fake, "").expect("delivered");
    assert_eq!(label, REWRITE_LABEL);
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Matched));
    assert_eq!(fake.logged(), ["set()", "delete(0,3)"]);
}

#[test]
fn clearing_an_already_empty_field_whose_set_is_absent_is_satisfied() {
    let mut fake = Fake::holding("");
    fake.set = Script::Absent;
    let (_, outcome) = replace(&fake, "").expect("satisfied");
    assert_eq!(outcome, DeliveryOutcome::Satisfied);
    assert_eq!(fake.logged(), ["set()"]);
}

#[test]
fn type_inserts_once_at_the_caret_with_no_delete() {
    let mut fake = Fake::holding("abcdef");
    fake.caret = Ok(Some((3, vec![])));
    let (_, outcome) = typed(&fake, "XY").expect("delivered");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Matched));
    assert_eq!(fake.logged(), ["insert(3,2,XY)"]);
}

#[test]
fn type_over_a_selection_deletes_it_then_inserts_at_its_start() {
    let mut fake = Fake::holding("abcdef");
    fake.caret = Ok(Some((2, span(1, 2))));
    let (_, outcome) = typed(&fake, "Z").expect("delivered");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Matched));
    assert_eq!(fake.logged(), ["delete(1,2)", "insert(1,1,Z)"]);
}

#[test]
fn an_empty_selection_range_is_not_deleted() {
    let mut fake = Fake::holding("abc");
    fake.caret = Ok(Some((1, span(1, 1))));
    typed(&fake, "Z").expect("delivered");
    assert_eq!(fake.logged(), ["insert(1,1,Z)"]);
}

#[test]
fn an_unreadable_caret_writes_nothing_and_is_not_delivered() {
    let fake = Fake::holding("abc");
    let error = typed(&fake, "Z").expect_err("no caret");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(fake.logged().is_empty());
}

#[test]
fn a_failed_insert_after_a_landed_selection_delete_is_delivered_unverified() {
    let mut fake = Fake::holding("abcdef");
    fake.caret = Ok(Some((2, span(1, 2))));
    fake.insert = Script::Fail(invalid());
    let error = typed(&fake, "Z").expect_err("insert failed");
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert!(error.suggestion.expect("a hint").contains("Read the field"));
}

#[test]
fn a_failed_insert_with_nothing_deleted_stays_not_delivered() {
    let mut fake = Fake::holding("abc");
    fake.caret = Ok(Some((1, vec![])));
    fake.insert = Script::Fail(invalid());
    let error = typed(&fake, "Z").expect_err("insert failed");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn an_absent_insert_with_nothing_deleted_falls_through_as_not_delivered() {
    let mut fake = Fake::holding("abc");
    fake.caret = Ok(Some((1, vec![])));
    fake.insert = Script::Absent;
    let (_, outcome) = typed(&fake, "Z").expect("no error");
    assert_eq!(outcome, DeliveryOutcome::NotDelivered);
}

#[test]
fn an_unreadable_field_leaves_the_insert_unobserved() {
    let mut fake = Fake::holding("abc");
    fake.caret = Ok(Some((1, vec![])));
    *fake.text.borrow_mut() = None;
    let (_, outcome) = typed(&fake, "Z").expect("delivered");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Unread));
}

#[test]
fn an_insert_the_field_ignored_is_not_matched() {
    let mut fake = Fake::holding("abc");
    fake.caret = Ok(Some((1, vec![])));
    fake.insert = Script::Ignore;
    let (_, outcome) = typed(&fake, "Z").expect("delivered");
    assert_eq!(outcome, DeliveryOutcome::Delivered(Observed::Unchanged));
}

#[test]
fn an_emoji_is_sent_with_its_byte_length_not_one() {
    let mut fake = Fake::holding("");
    fake.caret = Ok(Some((0, vec![])));
    typed(&fake, "\u{1F600}").expect("delivered");
    assert_eq!(fake.logged(), ["insert(0,4,\u{1F600})"]);
}

#[test]
fn every_length_sent_is_the_whole_utf8_length_of_its_text() {
    let samples = [
        "a",
        "\u{e9}",
        "e\u{301}",
        "\u{1F600}",
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
        "\u{1F1E9}\u{1F1EA}",
        "\u{4E2D}\u{6587}",
    ];
    for sample in samples {
        let mut fake = Fake::holding("");
        fake.caret = Ok(Some((0, vec![])));
        typed(&fake, sample).expect("delivered");
        let expected = format!("insert(0,{},{sample})", sample.len());
        assert_eq!(fake.logged(), [expected], "{sample:?}");
    }
}

#[test]
fn the_expected_text_counts_code_points_not_bytes() {
    assert_eq!(
        expected_after("a\u{1F600}b", 2..2, "X").as_deref(),
        Some("a\u{1F600}Xb")
    );
    assert_eq!(
        expected_after("a\u{1F600}b", 1..2, "X").as_deref(),
        Some("aXb")
    );
}

#[test]
fn an_out_of_range_replacement_has_no_expected_text() {
    assert_eq!(expected_after("ab", 1..5, "X"), None);
    assert_eq!(expected_after("ab", Range { start: 2, end: 1 }, "X"), None);
}

#[test]
fn a_read_only_bit_or_a_missing_editable_bit_refuses_the_write() {
    assert!(!writable(Some(true), Some(true)));
    assert!(!writable(Some(false), Some(false)));
    assert!(!writable(None, Some(false)));
}

#[test]
fn an_editable_field_or_unread_states_are_writable() {
    assert!(writable(Some(false), Some(true)));
    assert!(writable(None, None));
}

#[test]
fn the_read_only_refusal_is_action_not_supported_and_not_delivered() {
    let error = read_only_field();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}
