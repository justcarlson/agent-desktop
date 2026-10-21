const COMM_LEN: usize = 15;

pub(super) const PROTECTED_NAMES: &[&str] = &[
    "gnome-shell",
    "gnome-session",
    "gnome-session-binary",
    "Xorg",
    "Xwayland",
    "mutter",
    "gdm",
    "gdm-x-session",
    "gdm-wayland-session",
    "systemd",
    "dbus-daemon",
    "dbus-broker",
    "dbus-broker-launch",
    "at-spi-bus-launcher",
    "at-spi2-registryd",
    "pipewire",
    "wireplumber",
    "pulseaudio",
    "gnome-keyring-daemon",
    "polkit-gnome-authentication-agent-1",
    "polkitd",
    "openbox",
    "agent-desktop-linux-clipboard-owner",
];

pub(super) const PROTECTED_FAMILIES: &[&str] = &["gsd-", "xdg-desktop-portal"];

fn basename(identifier: &str) -> &str {
    identifier.rsplit('/').next().unwrap_or(identifier)
}

fn truncated(name: &str) -> String {
    name.chars().take(COMM_LEN).collect()
}

/// Whether `identifier` names an entry of `table`: equal ignoring case, after
/// a path is cut to its last component, or equal to the entry cut to the
/// 15 characters `comm` keeps. A name that merely contains an entry, or
/// starts with one, is a different process.
pub(super) fn matches_table(identifier: &str, table: &[&str]) -> bool {
    let name = basename(identifier.trim());
    if name.is_empty() {
        return false;
    }
    table.iter().any(|entry| {
        name.eq_ignore_ascii_case(entry) || name.eq_ignore_ascii_case(&truncated(entry))
    })
}

/// Whether `identifier` belongs to a named family: it starts with the family
/// prefix, or is the 15 characters `comm` keeps of a name that does.
pub(super) fn matches_family(identifier: &str, families: &[&str]) -> bool {
    let name = basename(identifier.trim()).to_ascii_lowercase();
    families.iter().any(|family| {
        name.starts_with(family) || (name.len() == COMM_LEN && family.starts_with(name.as_str()))
    })
}

pub(crate) fn is_protected(identifier: &str) -> bool {
    matches_table(identifier, PROTECTED_NAMES) || matches_family(identifier, PROTECTED_FAMILIES)
}

#[cfg(test)]
#[path = "protected_tests.rs"]
mod tests;
