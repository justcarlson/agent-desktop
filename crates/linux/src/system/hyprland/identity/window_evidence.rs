/// What a listed window says about itself: its process and that process's
/// instance token, its title and its size. A compositor-only window is
/// identified by its id, so only its process evidence is compared.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WindowEvidence {
    pub(crate) pid: u32,
    pub(crate) instance: Option<String>,
    pub(crate) title: String,
    pub(crate) size: Option<(f64, f64)>,
}
