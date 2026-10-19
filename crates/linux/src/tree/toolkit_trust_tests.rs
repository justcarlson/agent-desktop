use super::*;

fn toolkit(name: Option<&str>, version: Option<&str>) -> ToolkitId {
    ToolkitId {
        name: name.map(str::to_owned),
        version: version.map(str::to_owned),
    }
}

#[test]
fn gtk_three_qt_and_chromium_report_global_positions() {
    assert!(reports_global_position(&toolkit(
        Some("gtk"),
        Some("3.24.41")
    )));
    assert!(reports_global_position(&toolkit(Some("Qt"), Some("6.4"))));
    assert!(reports_global_position(&toolkit(Some("Chromium"), None)));
}

#[test]
fn gtk_four_an_unversioned_gtk_and_unknown_toolkits_do_not() {
    assert!(!reports_global_position(&toolkit(
        Some("GTK"),
        Some("4.14.5")
    )));
    assert!(!reports_global_position(&toolkit(Some("gtk"), None)));
    assert!(!reports_global_position(&toolkit(Some("gtk"), Some("x"))));
    assert!(!reports_global_position(&toolkit(
        Some("clutter"),
        Some("1.0")
    )));
    assert!(!reports_global_position(&toolkit(None, None)));
}

#[test]
fn same_window_landings_are_trusted_for_gtk_three_and_qt_only() {
    assert!(trusts_same_window_landing(&toolkit(
        Some("gtk"),
        Some("3.24")
    )));
    assert!(trusts_same_window_landing(&toolkit(
        Some("Qt"),
        Some("6.11")
    )));
}

#[test]
fn gtk_four_chromium_and_unknown_toolkits_are_untrusted_for_same_window_landings() {
    assert!(!trusts_same_window_landing(&toolkit(
        Some("gtk"),
        Some("4.14")
    )));
    assert!(!trusts_same_window_landing(&toolkit(
        Some("Chromium"),
        Some("1.0")
    )));
    assert!(!trusts_same_window_landing(&toolkit(Some("gtk"), None)));
    assert!(!trusts_same_window_landing(&toolkit(None, None)));
}

#[test]
fn descents_are_trusted_for_the_four_measured_toolkits() {
    assert!(trusts_descent(&toolkit(Some("gtk"), Some("4.14"))));
    assert!(trusts_descent(&toolkit(Some("Chromium"), Some("1.0"))));
    assert!(!trusts_descent(&toolkit(Some("gtk"), None)));
    assert!(!trusts_descent(&toolkit(Some("clutter"), Some("1.0"))));
}

#[test]
fn only_qt_counts_the_insert_length_in_utf16_units() {
    use super::insert_length_unit;
    use crate::tree::text_units::LengthUnit;
    let named = |name: &str, version: &str| toolkit(Some(name), Some(version));
    assert_eq!(
        insert_length_unit(&named("Qt", "6.4")),
        LengthUnit::Utf16Units
    );
    assert_eq!(
        insert_length_unit(&named("gtk", "3.24")),
        LengthUnit::Utf8Bytes
    );
    assert_eq!(
        insert_length_unit(&named("GTK", "4.14")),
        LengthUnit::Utf8Bytes
    );
    assert_eq!(
        insert_length_unit(&ToolkitId::default()),
        LengthUnit::Utf8Bytes
    );
}
