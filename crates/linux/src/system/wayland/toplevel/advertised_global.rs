/// One global from the registry: its name, interface and version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdvertisedGlobal {
    pub(crate) name: u32,
    pub(crate) interface: String,
    pub(crate) version: u32,
}
