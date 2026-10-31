use super::focus_activation::FocusActivation;
use super::window_info::WindowInfo;

/// The focused window and how focus was reached and proven.
#[derive(Debug, Clone)]
pub struct FocusOutcome {
    pub window: WindowInfo,
    pub activation: FocusActivation,
}
