type Environment = Vec<(&'static str, String)>;

pub(super) fn qt_environment() -> Result<(String, Environment), String> {
    let home = std::env::var("HOME").map_err(|error| error.to_string())?;
    let python = format!("{home}/.local/ad-tools/qtvenv/bin/python");
    if !std::path::Path::new(&python).exists() {
        return Err(format!("the Qt test environment is missing at {python}"));
    }
    let libraries = format!("{home}/.local/ad-tools/root/usr/lib/x86_64-linux-gnu");
    let library_path = match std::env::var("LD_LIBRARY_PATH") {
        Ok(existing) if !existing.is_empty() => format!("{libraries}:{existing}"),
        _ => libraries,
    };
    let environment = vec![
        ("QT_ACCESSIBILITY", "1".to_owned()),
        ("QT_LINUX_ACCESSIBILITY_ALWAYS_ON", "1".to_owned()),
        ("QT_QPA_PLATFORM", "xcb".to_owned()),
        ("LD_LIBRARY_PATH", library_path),
    ];
    Ok((python, environment))
}
