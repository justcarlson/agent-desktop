/// The workspaces one monitor shows: its active workspace and, when one is
/// open over it, its special workspace, both by name. `id` is the monitor
/// id that clients name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MonitorWorkspaces {
    pub(crate) monitor: String,
    pub(crate) id: Option<i64>,
    pub(crate) active: String,
    pub(crate) special: Option<String>,
}
