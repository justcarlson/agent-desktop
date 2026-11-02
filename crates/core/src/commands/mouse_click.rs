use crate::{
    AppError, Modifier, MouseButton, MouseEvent, MouseEventKind, Point,
    adapter::PlatformAdapter,
    commands::{
        operation_response::operation_response,
        point_resolve::{require_cursor_policy, require_pointer_capability},
    },
    context::CommandContext,
};
use serde_json::{Value, json};

pub struct MouseClickArgs {
    pub x: f64,
    pub y: f64,
    pub button: MouseButton,
    pub count: u32,
    pub modifiers: Vec<Modifier>,
}

pub fn execute(
    args: MouseClickArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    require_cursor_policy(context, "mouse-click")?;
    require_pointer_capability(adapter)?;
    crate::validate_mouse_click_count(args.count)?;
    let _cursor = crate::cursor_overlay::hold_cursor(context, crate::Deadline::standard()?);
    let lease = crate::ref_action::ref_action_timed_lease::TimedLease::acquire(
        adapter,
        crate::Deadline::standard()?,
    )?;
    let point = Point {
        x: args.x,
        y: args.y,
    };
    point.validate()?;
    let result = crate::cursor_overlay::dispatch_mouse_event_with_cursor(
        adapter,
        context,
        MouseEvent {
            kind: MouseEventKind::Click { count: args.count },
            point,
            button: args.button,
            modifiers: args.modifiers,
        },
        true,
        &lease,
    );
    let mut response = operation_response(
        &result?,
        json!({ "x": args.x, "y": args.y, "count": args.count }),
    )?;
    lease.release_into(&mut response);
    Ok(response)
}

#[cfg(test)]
#[path = "mouse_click_tests.rs"]
mod tests;
