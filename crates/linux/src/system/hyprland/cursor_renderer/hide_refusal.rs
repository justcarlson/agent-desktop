use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

/// Why a cursor hide refused, before any input event or frame request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HideRefusal {
    /// A live renderer did not acknowledge `hide` in time.
    Timeout { pid: u32 },
    /// A renderer refused `hide`, or the holder or the layer stack could not
    /// be handled.
    Failed { cause: &'static str, detail: String },
    /// A live renderer of another build holds a record.
    Incompatible { pid: u32 },
    /// A cursor-namespace layer belongs to no renderer that hid.
    UnknownOverlay { pid: Option<u32>, output: String },
}

impl HideRefusal {
    fn kind(&self) -> &'static str {
        match self {
            Self::Timeout { .. } => "cursor_hide_timeout",
            Self::Failed { .. } => "cursor_hide_failed",
            Self::Incompatible { .. } => "cursor_renderer_incompatible",
            Self::UnknownOverlay { .. } => "cursor_unknown_overlay",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::Timeout { pid } => {
                format!("Cursor renderer {pid} did not acknowledge the hide in time")
            }
            Self::Failed { cause, .. } => format!("The agent cursors could not be hidden: {cause}"),
            Self::Incompatible { pid } => {
                format!("Cursor renderer {pid} runs another agent-desktop build")
            }
            Self::UnknownOverlay { output, .. } => format!(
                "An agent-desktop-cursor layer on {output} belongs to no renderer that hid its cursors"
            ),
        }
    }

    /// The not-delivered error for this refusal.
    pub(crate) fn to_error(&self) -> AdapterError {
        let code = match self {
            Self::Timeout { .. } => ErrorCode::Timeout,
            _ => ErrorCode::ActionFailed,
        };
        let mut details = json!({
            "kind": self.kind(),
            "physical_delivery_started": false,
            "retryable": false,
        });
        let mut error = AdapterError::new(code, self.message());
        match self {
            Self::Timeout { pid } | Self::Incompatible { pid } => details["pid"] = json!(pid),
            Self::Failed { cause, detail } => {
                details["cause"] = json!(cause);
                error = error.with_platform_detail(detail.clone());
            }
            Self::UnknownOverlay { pid, output } => {
                details["layer_pid"] = json!(pid);
                details["layer_output"] = json!(output);
            }
        }
        error
            .with_details(details)
            .with_disposition(DeliverySemantics::not_delivered())
            .with_suggestion(self.suggestion())
    }

    fn suggestion(&self) -> &'static str {
        match self {
            Self::Incompatible { .. } => {
                "End the other build's session or run 'cursor-overlay disable' from it, then retry"
            }
            Self::UnknownOverlay { .. } => {
                "Close the program that draws the agent-desktop-cursor layer, then retry"
            }
            Self::Timeout { .. } | Self::Failed { .. } => "Retry; nothing was sent",
        }
    }
}

#[cfg(test)]
#[path = "hide_refusal_tests.rs"]
mod tests;
