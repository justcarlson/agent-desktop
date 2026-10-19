#![expect(dead_code, reason = "stub the owning lane replaces")]

pub(crate) struct OwnerRecord {
    pub(crate) pid: u32,
    pub(crate) start_ticks: u64,
    pub(crate) display: String,
    pub(crate) window: u32,
    pub(crate) nonce: String,
}

pub(crate) fn records_for(_display: &str) -> Vec<OwnerRecord> {
    Vec::new()
}

pub(crate) fn is_live(_record: &OwnerRecord) -> bool {
    false
}
