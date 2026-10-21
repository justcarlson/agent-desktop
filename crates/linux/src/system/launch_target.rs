use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};

use super::app_alias::{AliasIndex, basename_lower};
use super::desktop_entry::{DesktopEntry, parse};
use super::desktop_exec::{ExecContext, expand, program_word};
use super::desktop_lookup::{absolute_dirs, current_dirs, find, is_trusted};

/// What a launch identifier names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    Path(PathBuf),
    Command(String),
    Entry(String),
}

/// The directories an identifier is looked up in.
pub(crate) struct Search {
    pub(crate) data_dirs: Vec<PathBuf>,
    pub(crate) path_dirs: Vec<PathBuf>,
}

impl Search {
    pub(crate) fn from_env() -> Self {
        let path = std::env::var("PATH").unwrap_or_default();
        Self {
            data_dirs: current_dirs(),
            path_dirs: absolute_dirs(&path).collect(),
        }
    }
}

/// The program to run, its arguments and the facts attach decisions read.
pub(crate) struct Plan {
    pub(crate) argv: Vec<String>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) keys: Vec<String>,
}

fn refuse(code: ErrorCode, message: String) -> AdapterError {
    AdapterError::new(code, message).with_disposition(DeliverySemantics::not_delivered())
}

fn not_found(identifier: &str) -> AdapterError {
    refuse(
        ErrorCode::AppNotFound,
        format!("'{identifier}' is not an executable, a desktop id or a desktop entry name"),
    )
    .with_suggestion("Run 'list-apps' for running applications, or pass an absolute path.")
}

fn invalid(message: String) -> AdapterError {
    refuse(ErrorCode::InvalidArgs, message)
}

fn is_executable(file: &Path) -> bool {
    std::fs::metadata(file)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// The first executable file with this name under the directories.
pub(crate) fn find_on_path(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|dir| dir.join(name))
        .find(|file| is_executable(file))
}

fn classify_path(identifier: &str, search: &Search) -> Result<Target, AdapterError> {
    let file = PathBuf::from(identifier);
    if let Some(id) = file
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".desktop"))
    {
        return if is_trusted(&file, &search.data_dirs) {
            Ok(Target::Entry(id.to_owned()))
        } else {
            Err(invalid(format!(
                "'{identifier}' is a desktop file outside the XDG data directories and is not launched"
            )))
        };
    }
    if file.is_file() {
        Ok(Target::Path(file))
    } else {
        Err(not_found(identifier))
    }
}

/// Names the target of an identifier. The order is: an absolute path, a
/// relative path (refused), an installed desktop id, an executable on `PATH`,
/// and last the exact `Name` of one desktop entry, ignoring case. Nothing is
/// guessed from part of a name, and a `Name` several entries share is
/// ambiguous.
pub(crate) fn classify(
    identifier: &str,
    search: &Search,
    index: &AliasIndex,
) -> Result<Target, AdapterError> {
    if identifier.trim().is_empty() || identifier.contains('\0') {
        return Err(invalid(
            "The launch identifier is empty or holds a NUL".to_owned(),
        ));
    }
    if identifier.starts_with('/') {
        return classify_path(identifier, search);
    }
    if identifier.contains('/') {
        return Err(invalid(format!(
            "'{identifier}' is a relative path; pass an absolute path, a command name or a desktop id"
        )));
    }
    if let Some(file) = find(identifier, &search.data_dirs) {
        let id = file.file_stem().and_then(|stem| stem.to_str());
        return Ok(Target::Entry(id.unwrap_or(identifier).to_owned()));
    }
    if find_on_path(identifier, &search.path_dirs).is_some() {
        return Ok(Target::Command(identifier.to_owned()));
    }
    let mut ids = index.ids_named(identifier);
    match ids.len() {
        0 => Err(not_found(identifier)),
        1 => Ok(Target::Entry(ids.remove(0))),
        _ => Err(AdapterError::ambiguous_target(format!(
            "'{identifier}' is the name of several desktop entries: {}",
            ids.join(", ")
        ))
        .with_disposition(DeliverySemantics::not_delivered())),
    }
}

/// Whether the entry can be started: it must be a visible application with a
/// command that needs no terminal, and its `TryExec` must exist.
pub(crate) fn check_entry(entry: &DesktopEntry, path_dirs: &[PathBuf]) -> Result<(), AdapterError> {
    let id = &entry.id;
    if !entry.traits.application || entry.traits.hidden {
        return Err(refuse(
            ErrorCode::AppNotFound,
            format!("Desktop entry '{id}' is hidden or is not an application"),
        ));
    }
    if entry.traits.terminal || entry.exec.is_none() {
        return Err(refuse(
            ErrorCode::ActionNotSupported,
            format!("Desktop entry '{id}' needs a terminal or has no Exec and cannot be launched"),
        ));
    }
    let missing = entry.try_exec.as_deref().is_some_and(|program| {
        if program.starts_with('/') {
            !is_executable(Path::new(program))
        } else {
            find_on_path(program, path_dirs).is_none()
        }
    });
    if missing {
        return Err(refuse(
            ErrorCode::AppNotFound,
            format!("The program of desktop entry '{id}' is not installed"),
        ));
    }
    Ok(())
}

fn resolve_program(program: &str, path_dirs: &[PathBuf]) -> Result<String, AdapterError> {
    if program.contains('/') {
        return Ok(program.to_owned());
    }
    find_on_path(program, path_dirs)
        .map(|file| file.to_string_lossy().into_owned())
        .ok_or_else(|| {
            refuse(
                ErrorCode::AppNotFound,
                format!("The program '{program}' is not on PATH"),
            )
        })
}

fn entry_plan(id: &str, arguments: &[String], search: &Search) -> Result<Plan, AdapterError> {
    let file = find(id, &search.data_dirs).ok_or_else(|| not_found(id))?;
    let text = std::fs::read_to_string(&file).map_err(|_| not_found(id))?;
    let entry = parse(id, &text).ok_or_else(|| {
        refuse(
            ErrorCode::AppNotFound,
            format!("Desktop file of '{id}' has no [Desktop Entry] group"),
        )
    })?;
    check_entry(&entry, &search.path_dirs)?;
    let exec = entry.exec.as_deref().unwrap_or_default();
    let program = program_word(exec).ok_or_else(|| {
        refuse(
            ErrorCode::ActionNotSupported,
            format!("Desktop entry '{id}' has an empty Exec"),
        )
    })?;
    let file_text = file.to_string_lossy();
    let mut argv = expand(
        exec,
        &ExecContext {
            arguments,
            icon: entry.icon.as_deref(),
            name: &entry.name,
            file: Some(&file_text),
        },
    );
    let at = argv.iter().position(|word| *word == program).unwrap_or(0);
    argv[at] = resolve_program(&program, &search.path_dirs)?;
    let keys = [Some(entry.id.to_lowercase())]
        .into_iter()
        .chain([basename_lower(&program)])
        .chain(entry.try_exec.as_deref().map(basename_lower))
        .chain([entry.traits.wm_class.as_deref().map(str::to_lowercase)])
        .flatten()
        .collect();
    Ok(Plan {
        argv,
        cwd: entry.path.map(PathBuf::from),
        keys,
    })
}

/// The command line and the identity keys of a target.
pub(crate) fn plan(
    target: &Target,
    arguments: &[String],
    search: &Search,
) -> Result<Plan, AdapterError> {
    match target {
        Target::Entry(id) => entry_plan(id, arguments, search),
        Target::Path(file) => Ok(Plan {
            argv: std::iter::once(file.to_string_lossy().into_owned())
                .chain(arguments.iter().cloned())
                .collect(),
            cwd: None,
            keys: basename_lower(&file.to_string_lossy())
                .into_iter()
                .collect(),
        }),
        Target::Command(name) => {
            let program = resolve_program(name, &search.path_dirs)?;
            Ok(Plan {
                argv: std::iter::once(program)
                    .chain(arguments.iter().cloned())
                    .collect(),
                cwd: None,
                keys: vec![name.to_lowercase()],
            })
        }
    }
}

#[cfg(test)]
#[path = "launch_target_tests.rs"]
mod tests;
