use serde_json::json;

use crate::{AdapterError, AppError, ErrorCode, PlatformAdapter, ProcessIdentity, WindowInfo};

pub(crate) fn focus_entry_window(
    entry: &crate::RefEntry,
    adapter: &dyn PlatformAdapter,
    context: &crate::CommandContext,
    lease: &crate::InteractionLease,
) -> Result<WindowInfo, AppError> {
    let process_instance = required_text(
        entry.process.process_instance.as_deref(),
        "Headed input requires target process-instance identity",
    )?;
    let window_id = required_text(
        entry.source.source_window_id.as_deref(),
        "Headed input requires an exact source window id",
    )?;
    let expected = WindowInfo {
        id: window_id.to_string(),
        title: entry.source.source_window_title.clone().unwrap_or_default(),
        app: entry.source.source_app.clone().unwrap_or_default(),
        pid: entry.process.pid,
        process_instance: Some(process_instance.to_string()),
        bounds: None,
        state: crate::WindowState::default(),
    };
    focus_exact_window(&expected, adapter, context, lease)
}

pub(crate) fn focus_process_window(
    process: ProcessIdentity,
    adapter: &dyn PlatformAdapter,
    context: &crate::CommandContext,
    lease: &crate::InteractionLease,
) -> Result<WindowInfo, AppError> {
    let expected =
        crate::system::window_lookup::find_window_for_process(process, adapter, lease.deadline())?;
    focus_exact_window(&expected, adapter, context, lease)
}

pub(crate) fn focus_exact_window(
    expected: &WindowInfo,
    adapter: &dyn PlatformAdapter,
    context: &crate::CommandContext,
    lease: &crate::InteractionLease,
) -> Result<WindowInfo, AppError> {
    let live = adapter.resolve_window_strict(expected, lease.deadline())?;
    if live.id != expected.id
        || live.pid != expected.pid
        || live.process_instance != expected.process_instance
    {
        return Err(AdapterError::stale_ref_because(
            "Headed target window belongs to a different process instance",
        )
        .into());
    }
    adapter
        .focus_window(&live, lease)
        .map_err(input_not_started)?;
    context.trace_lazy(
        "input.focus_window",
        || json!({ "pid": live.pid, "window_id": live.id, "ok": true }),
    )?;
    Ok(live)
}

/// An activation refused before any input event: the activation is a
/// side effect the details report, and the input itself was not delivered.
fn input_not_started(error: AdapterError) -> AdapterError {
    let started = error
        .details
        .as_ref()
        .and_then(|details| details.get("physical_delivery_started"))
        .and_then(serde_json::Value::as_bool);
    match started {
        Some(false) => error.with_disposition(crate::DeliverySemantics::not_delivered()),
        _ => error,
    }
}

fn required_text<'a>(value: Option<&'a str>, message: &str) -> Result<&'a str, AppError> {
    value.filter(|value| !value.is_empty()).ok_or_else(|| {
        AdapterError::new(ErrorCode::ActionNotSupported, message)
            .with_details(json!({ "physical_delivery_started": false }))
            .into()
    })
}

#[cfg(test)]
#[path = "headed_focus_tests.rs"]
mod tests;
