//! The R1 binding decision. One Hyprland instance is bound only when its IPC
//! socket answers and a Wayland socket served by the same compositor process
//! answers a registry round trip. An explicitly named instance is checked
//! alone; discovery reads every instance directory and refuses when more than
//! one answers. A silent endpoint refuses instead of being skipped, because
//! skipping it could bind a compositor the caller did not mean.

use std::path::{Path, PathBuf};

use agent_desktop_core::Deadline;

pub(crate) use super::bind_refusal::BindRefusal;
pub(crate) use super::bind_request::BindRequest;
pub(crate) use super::hyprland_binding::HyprlandBinding;
pub(crate) use super::ipc_answer::IpcAnswer;
pub(crate) use super::observe::Observe;
pub(crate) use super::wayland_answer::WaylandAnswer;

const IPC_SOCKET: &str = ".socket.sock";

struct Instance {
    signature: String,
    ipc_socket: PathBuf,
    pid: u32,
}

struct Pairing<'a> {
    request: &'a BindRequest,
    observe: &'a dyn Observe,
    runtime_dir: &'a Path,
    deadline: Deadline,
    explicit: bool,
}

/// Decides the binding. `Ok(None)` means no Hyprland instance serves this
/// session, so a Wayland session keeps the generic route.
pub(crate) fn bind(
    request: &BindRequest,
    observe: &dyn Observe,
    deadline: Deadline,
) -> Result<Option<HyprlandBinding>, BindRefusal> {
    let Some(runtime_dir) = request.runtime_dir.as_deref() else {
        return Ok(None);
    };
    let hypr_dir = runtime_dir.join("hypr");
    let mut pairing = Pairing {
        request,
        observe,
        runtime_dir,
        deadline,
        explicit: true,
    };
    if let Some(signature) = &request.signature
        && let Some(instance) = answering(observe, &hypr_dir, signature, deadline)?
    {
        return pairing.pair(instance);
    }
    pairing.explicit = false;
    let signatures = observe.signatures(&hypr_dir);
    if pairing.serves_no_instance(&hypr_dir, &signatures) {
        return Ok(None);
    }
    let mut live = Vec::new();
    for signature in signatures {
        if let Some(instance) = answering(observe, &hypr_dir, &signature, deadline)? {
            live.push(instance);
        }
    }
    match live.len() {
        0 => Ok(None),
        1 => pairing.pair(live.remove(0)),
        _ => pairing.several(live),
    }
}

fn answering(
    observe: &dyn Observe,
    hypr_dir: &Path,
    signature: &str,
    deadline: Deadline,
) -> Result<Option<Instance>, BindRefusal> {
    let ipc_socket = hypr_dir.join(signature).join(IPC_SOCKET);
    match observe.ipc(&ipc_socket, deadline) {
        IpcAnswer::Live(pid) => Ok(Some(Instance {
            signature: signature.to_owned(),
            ipc_socket,
            pid,
        })),
        IpcAnswer::Gone => Ok(None),
        IpcAnswer::Silent => Err(BindRefusal::Silent { socket: ipc_socket }),
    }
}

impl Pairing<'_> {
    /// True when the named Wayland socket's server owns no instance's IPC
    /// socket, so no instance can bind and none is waited on.
    fn serves_no_instance(&self, hypr_dir: &Path, signatures: &[String]) -> bool {
        let Some(socket) = &self.request.wayland_socket else {
            return false;
        };
        let Some(owner) = self.observe.peer(socket) else {
            return false;
        };
        !signatures.iter().any(|signature| {
            self.observe
                .peer(&hypr_dir.join(signature).join(IPC_SOCKET))
                == Some(owner)
        })
    }

    fn several(&self, live: Vec<Instance>) -> Result<Option<HyprlandBinding>, BindRefusal> {
        if let Some(socket) = &self.request.wayland_socket {
            let peer = self.observe.peer(socket);
            if !live.iter().any(|instance| Some(instance.pid) == peer) {
                return Ok(None);
            }
        }
        Err(BindRefusal::Ambiguous {
            signatures: live
                .into_iter()
                .map(|instance| instance.signature)
                .collect(),
        })
    }

    fn pair(&self, instance: Instance) -> Result<Option<HyprlandBinding>, BindRefusal> {
        match &self.request.wayland_socket {
            Some(socket) => self.pair_named(instance, socket),
            None => self.pair_discovered(instance),
        }
    }

    fn pair_named(
        &self,
        instance: Instance,
        socket: &Path,
    ) -> Result<Option<HyprlandBinding>, BindRefusal> {
        match self.observe.wayland(socket, instance.pid, self.deadline) {
            WaylandAnswer::Live { mapping, .. } => self.finish(instance, socket, mapping),
            WaylandAnswer::Silent => Err(BindRefusal::Silent {
                socket: socket.to_path_buf(),
            }),
            WaylandAnswer::Other(_) if !self.explicit => Ok(None),
            WaylandAnswer::Other(pid) => Err(mismatch(&instance, socket, Some(pid))),
            WaylandAnswer::Gone => Err(mismatch(&instance, socket, None)),
        }
    }

    fn pair_discovered(&self, instance: Instance) -> Result<Option<HyprlandBinding>, BindRefusal> {
        let own = self
            .observe
            .wayland_sockets(self.runtime_dir)
            .into_iter()
            .filter(|socket| self.observe.peer(socket) == Some(instance.pid));
        for socket in own {
            match self.observe.wayland(&socket, instance.pid, self.deadline) {
                WaylandAnswer::Live { mapping, .. } => {
                    return self.finish(instance, &socket, mapping);
                }
                WaylandAnswer::Silent => return Err(BindRefusal::Silent { socket }),
                WaylandAnswer::Other(_) | WaylandAnswer::Gone => {}
            }
        }
        Err(BindRefusal::Mismatch {
            detail: format!(
                "no Wayland socket in {} is served by Hyprland instance {} (pid {})",
                self.runtime_dir.display(),
                instance.signature,
                instance.pid
            ),
        })
    }

    fn finish(
        &self,
        instance: Instance,
        socket: &Path,
        mapping: bool,
    ) -> Result<Option<HyprlandBinding>, BindRefusal> {
        if !mapping {
            return Ok(None);
        }
        let Some(start_time) = self.observe.start_time(instance.pid) else {
            return Ok(None);
        };
        Ok(Some(HyprlandBinding {
            signature: instance.signature,
            ipc_socket: instance.ipc_socket,
            wayland_socket: socket.to_path_buf(),
            pid: instance.pid,
            start_time,
        }))
    }
}

fn mismatch(instance: &Instance, socket: &Path, peer: Option<u32>) -> BindRefusal {
    let served = peer.map_or_else(
        || "does not answer".to_owned(),
        |pid| format!("is served by pid {pid}"),
    );
    BindRefusal::Mismatch {
        detail: format!(
            "Hyprland instance {} (pid {}) is bound, but the Wayland socket {} {served}",
            instance.signature,
            instance.pid,
            socket.display()
        ),
    }
}

#[cfg(test)]
#[path = "hyprland_bind_tests.rs"]
mod tests;
