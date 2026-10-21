use super::reports_global_position;

#[test]
fn gtk_four_is_window_relative_and_gtk_three_is_global() {
    assert!(!reports_global_position(Some("GTK"), Some("4.14.5")));
    assert!(reports_global_position(Some("gtk"), Some("3.24.41")));
}

#[test]
fn qt_and_chromium_are_global() {
    assert!(reports_global_position(Some("Qt"), Some("6.4")));
    assert!(reports_global_position(Some("Chromium"), None));
}

#[test]
fn an_unreadable_or_unknown_toolkit_is_not_trusted() {
    assert!(!reports_global_position(None, None));
    assert!(!reports_global_position(Some("gtk"), None));
    assert!(!reports_global_position(Some("gtk"), Some("x")));
    assert!(!reports_global_position(Some("clutter"), Some("1.0")));
}
