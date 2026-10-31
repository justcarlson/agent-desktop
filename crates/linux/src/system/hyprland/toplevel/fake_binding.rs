use crate::system::hyprland::bind::hyprland_binding::HyprlandBinding;
use crate::system::process::proc_stat::read_stat;
use crate::system::wayland::fake_compositor::FakeCompositor;

/// A binding whose Wayland socket is `compositor` and whose compositor
/// process is this test process, which serves the fake socket.
pub(crate) fn binding_for(compositor: &FakeCompositor) -> HyprlandBinding {
    let pid = std::process::id();
    let start_time = read_stat(pid)
        .ok()
        .flatten()
        .map(|stat| stat.start_time)
        .expect("start time");
    HyprlandBinding {
        signature: "sig_1".into(),
        ipc_socket: compositor.socket.with_extension("ipc"),
        wayland_socket: compositor.socket.clone(),
        pid,
        start_time,
    }
}
