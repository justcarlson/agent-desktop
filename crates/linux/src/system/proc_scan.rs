use super::app_name::name_matches;
use super::process_facts;

/// A running process whose `comm` or executable basename is `query`, found
/// by scanning `/proc`: its process id and `comm`.
pub(crate) fn find_running(query: &str) -> Option<(u32, String)> {
    std::fs::read_dir("/proc")
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
        .find_map(|pid| {
            let facts = process_facts::read(pid)?;
            name_matches(query, &facts.comm, &facts).then(|| (pid, facts.comm.clone()))
        })
}
