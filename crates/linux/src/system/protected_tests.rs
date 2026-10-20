use super::{PROTECTED_FAMILIES, PROTECTED_NAMES, is_protected, matches_family, matches_table};

const TABLE: &[&str] = &["gnome-shell", "gnome-session-binary", "Xorg"];

#[test]
fn an_entry_matches_by_exact_name_uppercase_and_path() {
    for entry in TABLE {
        assert!(matches_table(entry, TABLE), "{entry}");
        assert!(matches_table(&entry.to_uppercase(), TABLE), "{entry}");
        assert!(
            matches_table(&format!("/usr/bin/{entry}"), TABLE),
            "{entry}"
        );
        assert!(matches_table(&format!("./{entry}"), TABLE), "{entry}");
    }
}

#[test]
fn a_long_entry_matches_its_fifteen_character_comm_form() {
    assert!(matches_table("gnome-session-b", TABLE));
    assert!(!matches_table("gnome-session-c", TABLE));
}

#[test]
fn a_name_that_only_contains_or_extends_an_entry_does_not_match() {
    for other in [
        "gnome-shell-calendar-server",
        "gnome-text-editor",
        "gnome",
        "shell",
        "xorg-helper",
        "my-gnome-shell",
    ] {
        assert!(!matches_table(other, TABLE), "{other}");
    }
}

#[test]
fn an_empty_identifier_matches_nothing() {
    assert!(!matches_table("", TABLE));
    assert!(!matches_table("  ", TABLE));
    assert!(!matches_table("/", TABLE));
    assert!(!matches_table("", &[""]));
}

#[test]
fn a_path_to_another_binary_in_an_entrys_directory_does_not_match() {
    assert!(!matches_table("/usr/bin/gnome-shell/other", TABLE));
}

#[test]
fn every_entry_of_the_real_table_matches_in_every_form() {
    for entry in PROTECTED_NAMES {
        assert!(is_protected(entry), "{entry}");
        assert!(is_protected(&entry.to_uppercase()), "{entry}");
        assert!(is_protected(&format!("/usr/libexec/{entry}")), "{entry}");
        assert!(is_protected(&entry.chars().take(15).collect::<String>()));
        assert!(
            !is_protected(&format!("{entry}-calendar-server")),
            "{entry}"
        );
    }
    assert!(!is_protected(""));
}

#[test]
fn a_family_matches_by_prefix_and_by_its_comm_form_only() {
    let families = &["gsd-", "xdg-desktop-portal"];
    for member in [
        "gsd-power",
        "GSD-MEDIA-KEYS",
        "/usr/libexec/gsd-color",
        "xdg-desktop-portal",
        "xdg-desktop-portal-gtk",
        "xdg-desktop-por",
    ] {
        assert!(matches_family(member, families), "{member}");
    }
    for other in [
        "gsd",
        "my-gsd-power",
        "xdg-desktop",
        "xdg-open",
        "xdg-desktop-pxx",
        "",
    ] {
        assert!(!matches_family(other, families), "{other}");
    }
}

#[test]
fn the_real_families_protect_their_members() {
    for family in PROTECTED_FAMILIES {
        assert!(is_protected(&format!("{family}x")), "{family}");
    }
    assert!(is_protected("gsd-power"));
    assert!(!is_protected("gnome-text-editor"));
    assert!(!is_protected("gnome-calculator"));
}
