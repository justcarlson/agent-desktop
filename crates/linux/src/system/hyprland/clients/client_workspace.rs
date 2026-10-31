/// The workspace a client is on, by name, and the id of the monitor that
/// shows it. A reply without a monitor id leaves `monitor` empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClientWorkspace {
    pub(crate) name: String,
    pub(crate) monitor: Option<i64>,
}
