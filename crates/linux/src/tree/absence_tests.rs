use agent_desktop_core::ErrorCode;

use super::*;
use crate::system::{NAME_HAS_NO_OWNER, SERVICE_UNKNOWN};
use crate::tree::test_support::{client_timeout, method_error};

const FAILED: &str = "org.freedesktop.DBus.Error.Failed";
const INVALID_ARGS: &str = "org.freedesktop.DBus.Error.InvalidArgs";

fn gone_code(error: &zbus::Error, liveness: Liveness) -> Option<ErrorCode> {
    gone_error(error, liveness).map(|error| error.code)
}

#[test]
fn an_exited_application_is_gone_on_whatever_call_reports_it() {
    for name in [SERVICE_UNKNOWN, NAME_HAS_NO_OWNER] {
        let error = method_error(name);
        assert_eq!(failure(&error), Failure::AppGone);
        for liveness in [Liveness::Alive, Liveness::TimedOut, Liveness::ObjectGone] {
            assert_eq!(gone_code(&error, liveness), Some(ErrorCode::AppNotFound));
        }
    }
}

#[test]
fn a_node_whose_role_call_fails_in_the_suspect_family_is_a_destroyed_object() {
    for name in [UNKNOWN_OBJECT, UNKNOWN_METHOD, UNKNOWN_INTERFACE] {
        let role = method_error(name);
        assert_eq!(Liveness::of(Some(&role)), Liveness::ObjectGone);
    }
}

#[test]
fn every_read_of_a_destroyed_gtk4_object_is_the_object_gone() {
    let liveness = Liveness::of(Some(&method_error(UNKNOWN_METHOD)));
    let read = method_error(UNKNOWN_METHOD);
    assert_eq!(gone_code(&read, liveness), Some(ErrorCode::ElementNotFound));
}

#[test]
fn a_destroyed_qt_object_answers_methods_and_properties_with_different_names() {
    let liveness = Liveness::of(Some(&method_error(UNKNOWN_OBJECT)));
    for name in [UNKNOWN_OBJECT, UNKNOWN_INTERFACE] {
        assert_eq!(
            gone_code(&method_error(name), liveness),
            Some(ErrorCode::ElementNotFound)
        );
    }
}

#[test]
fn the_suspect_names_on_a_node_whose_role_answered_leave_the_node_alive() {
    let liveness = Liveness::of(None);
    assert_eq!(liveness, Liveness::Alive);
    let live_reads = [
        (UNKNOWN_OBJECT, "Qt Text.GetText"),
        (UNKNOWN_INTERFACE, "Qt Properties.Get(Value)"),
        (UNKNOWN_METHOD, "GTK4 Text.GetText"),
        (INVALID_ARGS, "GTK4 Properties.Get(Value)"),
        (FAILED, "GTK3 Properties.Get(Value)"),
    ];
    for (name, read) in live_reads {
        assert_eq!(gone_code(&method_error(name), liveness), None, "{read}");
    }
}

#[test]
fn a_stale_object_that_still_answers_is_alive_to_its_own_reads() {
    assert_eq!(Liveness::of(None), Liveness::Alive);
    assert_eq!(
        gone_code(&method_error(UNKNOWN_OBJECT), Liveness::Alive),
        None
    );
}

#[test]
fn a_role_call_that_times_out_never_makes_the_object_gone() {
    let timeout = client_timeout();
    assert_eq!(failure(&timeout), Failure::TimedOut);
    let liveness = Liveness::of(Some(&timeout));
    assert_eq!(liveness, Liveness::TimedOut);
    for name in [UNKNOWN_OBJECT, UNKNOWN_METHOD, UNKNOWN_INTERFACE] {
        assert_eq!(gone_code(&method_error(name), liveness), None);
    }
    assert_eq!(gone_code(&timeout, liveness), None);
}

#[test]
fn an_error_outside_the_known_names_is_a_failed_field_and_never_gone() {
    for name in [FAILED, INVALID_ARGS, "org.example.Error.Unheard"] {
        let error = method_error(name);
        assert_eq!(failure(&error), Failure::Other);
        assert_eq!(Liveness::of(Some(&error)), Liveness::Alive);
        assert_eq!(gone_code(&error, Liveness::Alive), None);
    }
}

#[test]
fn a_role_that_cannot_be_decoded_is_not_a_destroyed_object() {
    let undecodable = zbus::Error::InvalidReply;
    assert_eq!(Liveness::of(Some(&undecodable)), Liveness::Alive);
}

#[test]
fn a_gone_application_outranks_the_object_verdict() {
    assert_eq!(
        read_failure(Failure::Suspect, Liveness::AppGone),
        ReadFailure::AppGone
    );
    assert_eq!(
        read_failure(Failure::Other, Liveness::ObjectGone),
        ReadFailure::ObjectGone
    );
}
