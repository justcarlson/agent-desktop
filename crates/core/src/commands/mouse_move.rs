use crate::{
    AppError, MouseButton, MouseEvent, MouseEventKind, Point,
    adapter::PlatformAdapter,
    commands::{
        operation_response::operation_response,
        point_resolve::{require_cursor_policy, require_pointer_capability},
    },
    context::CommandContext,
};
use serde_json::{Value, json};

pub struct MouseMoveArgs {
    pub x: f64,
    pub y: f64,
}

pub fn execute(
    args: MouseMoveArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    require_cursor_policy(context, "mouse-move")?;
    require_pointer_capability(adapter)?;
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
            kind: MouseEventKind::Move,
            point,
            button: MouseButton::Left,
            modifiers: Vec::new(),
        },
        false,
        &lease,
    );
    let mut response = operation_response(&result?, json!({ "x": args.x, "y": args.y }))?;
    lease.release_into(&mut response);
    Ok(response)
}
