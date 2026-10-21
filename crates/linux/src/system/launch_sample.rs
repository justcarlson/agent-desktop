use super::running_app::Running;
use std::collections::HashSet;
use std::process::ExitStatus;

/// What one look at the registry shows about a launch.
pub(crate) struct Sample<'a> {
    pub(crate) child_pid: u32,
    pub(crate) child: Option<ExitStatus>,
    pub(crate) apps: &'a [Running],
    pub(crate) before: &'a HashSet<u32>,
    pub(crate) keys: &'a [String],
    pub(crate) settled: bool,
}
