use agent_desktop_core::{AdapterError, ErrorCode};

use super::read_if_registered;

#[test]
fn a_watcher_that_vanishes_during_the_read_is_not_treated_as_an_empty_list() {
    let error = read_if_registered(None, || -> Result<Vec<String>, AdapterError> {
        panic!("the watcher property must not be read when the owner is absent")
    })
    .expect_err("watcher is absent");

    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(error.details.expect("watcher state")["watcher"], "absent");
    assert!(
        error
            .suggestion
            .as_deref()
            .unwrap_or_default()
            .contains("AppIndicator")
    );
}

#[test]
fn a_registered_watcher_can_have_an_empty_list() {
    assert!(
        read_if_registered(Some(":1.8".to_owned()), || Ok(Vec::<String>::new()))
            .expect("empty item list")
            .is_empty()
    );
}
