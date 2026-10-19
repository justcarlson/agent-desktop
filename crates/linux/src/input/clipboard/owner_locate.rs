use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};

pub(crate) const HELPER_NAME: &str = "agent-desktop-linux-clipboard-owner";

const WRITABLE_BY_OTHERS: u32 = 0o022;
const ANY_EXECUTE: u32 = 0o111;
const DELETED_SUFFIX: &str = " (deleted)";

fn refusal(message: String) -> AdapterError {
    AdapterError::new(ErrorCode::PlatformNotSupported, message)
        .with_suggestion(format!(
            "Install {HELPER_NAME} in the directory of the agent-desktop binary or library, owned by this user and not writable by others"
        ))
        .with_disposition(DeliverySemantics::not_delivered())
}

/// The file that holds `address` according to a `/proc/<pid>/maps` listing.
pub(crate) fn image_from_maps(maps: &str, address: usize) -> Option<PathBuf> {
    maps.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let range = fields.next()?;
        let path = fields.skip(4).collect::<Vec<_>>().join(" ");
        let path = path.as_str();
        let (start, end) = range.split_once('-')?;
        let start = usize::from_str_radix(start, 16).ok()?;
        let end = usize::from_str_radix(end, 16).ok()?;
        if !(start..end).contains(&address) || !path.starts_with('/') {
            return None;
        }
        Some(PathBuf::from(
            path.strip_suffix(DELETED_SUFFIX).unwrap_or(path),
        ))
    })
}

fn check_helper(helper: &Path) -> Result<(), AdapterError> {
    let metadata = fs::symlink_metadata(helper).map_err(|error| {
        refusal(format!(
            "The clipboard owner helper {} is missing: {error}",
            helper.display()
        ))
    })?;
    let owner_ok = metadata.uid() == rustix::process::getuid().as_raw() || metadata.uid() == 0;
    let mode = metadata.permissions().mode();
    let problem = if !metadata.file_type().is_file() {
        Some("is not a regular file")
    } else if mode & ANY_EXECUTE == 0 {
        Some("is not executable")
    } else if !owner_ok {
        Some("is owned by another user")
    } else if mode & WRITABLE_BY_OTHERS != 0 {
        Some("is writable by group or others")
    } else {
        None
    };
    match problem {
        Some(problem) => Err(refusal(format!(
            "The clipboard owner helper {} {problem}",
            helper.display()
        ))),
        None => Ok(()),
    }
}

/// The helper that sits beside the image `maps` shows holding `address`.
pub(crate) fn locate_in(maps: &str, address: usize) -> Result<PathBuf, AdapterError> {
    let image = image_from_maps(maps, address)
        .and_then(|image| fs::canonicalize(image).ok())
        .ok_or_else(|| {
            refusal("The image that holds the clipboard code could not be found".to_owned())
        })?;
    let helper = image
        .parent()
        .ok_or_else(|| refusal("The image that holds the clipboard code has no directory".into()))?
        .join(HELPER_NAME);
    check_helper(&helper)?;
    Ok(helper)
}

pub(crate) fn locate_helper() -> Result<PathBuf, AdapterError> {
    let maps = fs::read_to_string("/proc/self/maps")
        .map_err(|error| refusal(format!("The loaded images could not be listed: {error}")))?;
    locate_in(&maps, locate_helper as usize)
}

#[cfg(test)]
#[path = "owner_locate_tests.rs"]
mod tests;
