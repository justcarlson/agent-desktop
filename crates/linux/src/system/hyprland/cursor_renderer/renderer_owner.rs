use std::path::PathBuf;

/// The process that owns a renderer record: its pid, its start time in clock
/// ticks (`/proc/<pid>/stat` field 22), the executable it runs, and the
/// random generation that tells this renderer apart from any later one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RendererOwner {
    pub(crate) pid: u32,
    pub(crate) start_time: u64,
    pub(crate) exe: PathBuf,
    pub(crate) generation: String,
}

impl RendererOwner {
    /// The owner record for this process with a fresh generation.
    pub(crate) fn current() -> Option<Self> {
        let pid = std::process::id();
        let start_time = crate::system::process::proc_stat::read_stat(pid)
            .ok()
            .flatten()?
            .start_time;
        Some(Self {
            pid,
            start_time,
            exe: std::fs::read_link("/proc/self/exe").ok()?,
            generation: fresh_generation()?,
        })
    }
}

/// A random 128-bit id in lowercase hex.
pub(crate) fn fresh_generation() -> Option<String> {
    use std::io::Read;
    let mut bytes = [0_u8; 16];
    std::fs::File::open("/dev/urandom")
        .ok()?
        .read_exact(&mut bytes)
        .ok()?;
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
