use agent_desktop_core::{AdapterError, ErrorCode};

use super::tray_observation::read;
use crate::tray::view::TrayView;

#[test]
fn an_absent_watcher_refuses_the_tray_snapshot_before_reading_items() {
    let error = read(
        || {
            Err(AdapterError::new(
                ErrorCode::PlatformNotSupported,
                "No system tray watcher is running in this session",
            )
            .with_suggestion("Enable the AppIndicator extension so tray items register"))
        },
        || panic!("a missing watcher must not be read as an empty tray"),
    )
    .err()
    .expect("tray observation requires its watcher");

    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert!(error.message.contains("watcher"));
    assert!(
        error
            .suggestion
            .as_deref()
            .unwrap_or_default()
            .contains("AppIndicator")
    );
}

#[test]
fn an_unreadable_watcher_error_is_not_replaced_with_an_empty_tray() {
    let error = read(
        || {
            Err(AdapterError::new(ErrorCode::Timeout, "watcher unreadable")
                .with_details(serde_json::json!({ "retryable": true })))
        },
        || panic!("an unreadable watcher must not be read as an empty tray"),
    )
    .err()
    .expect("tray observation requires a readable watcher");

    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(error.details.expect("retry details")["retryable"] == true);
}

#[test]
fn a_present_empty_watcher_returns_a_supported_empty_view() {
    let view = read(|| Ok(()), || Ok(TrayView { items: Vec::new() }))
        .expect("a readable empty watcher is supported");

    assert!(view.items.is_empty());
}

#[test]
fn tray_item_read_failures_propagate() {
    let error = read(
        || Ok(()),
        || Err(AdapterError::new(ErrorCode::Timeout, "tray read failed")),
    )
    .err()
    .expect("a failed tray read is not an empty tray");

    assert_eq!(error.message, "tray read failed");
}
