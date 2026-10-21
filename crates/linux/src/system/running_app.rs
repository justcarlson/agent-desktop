/// A running application as the registry lists it, once per process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Running {
    pub(crate) pid: u32,
    pub(crate) instance: String,
    pub(crate) name: String,
    pub(crate) registry_name: Option<String>,
    pub(crate) comm: String,
}
