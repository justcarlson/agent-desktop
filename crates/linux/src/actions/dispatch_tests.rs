use agent_desktop_core::{Action, ActionRequest, Deadline, ErrorCode, NativeHandle, SystemOps};

use super::execute_action;
use crate::adapter::LinuxAdapter;

#[test]
fn a_foreign_handle_is_invalid_args_before_any_arm_runs() {
    let adapter = LinuxAdapter::new();
    let deadline = Deadline::standard().expect("a standard deadline builds");
    let lease = SystemOps::acquire_interaction_lease(&adapter, deadline)
        .expect("the shared lease is granted");
    let foreign = NativeHandle::new(5u32);
    let error = execute_action(
        adapter.bus(),
        &foreign,
        ActionRequest::headless(Action::Click),
        &lease,
    )
    .expect_err("a foreign handle is refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
}
