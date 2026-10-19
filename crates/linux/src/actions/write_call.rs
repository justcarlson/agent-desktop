use agent_desktop_core::AdapterError;
use zbus::export::serde::Serialize;
use zbus::zvariant::DynamicType;

use super::context::Target;
use super::mutation::{WriteKind, WriteReply};

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn invoke<B: Serialize + DynamicType>(
    _target: &Target<'_>,
    _interface: &'static str,
    _method: &'static str,
    _body: &B,
    _kind: WriteKind,
    _advertised: bool,
) -> Result<WriteReply, AdapterError> {
    Ok(WriteReply::Absent)
}

#[cfg(test)]
#[path = "write_call_tests.rs"]
mod tests;
