use std::io::{self, ErrorKind};
use std::os::unix::net::UnixStream;
use std::path::Path;

use super::display_server::{Probe, Reach};

const X11_SOCKET_DIR: &str = "/tmp/.X11-unix";

/// Probes display endpoints by opening and dropping a connection to their
/// unix sockets, which neither server treats as a client.
pub(crate) struct SocketProbe;

fn reach_of(result: io::Result<UnixStream>) -> Reach {
    match result {
        Ok(_) => Reach::Connected,
        Err(error) if error.kind() == ErrorKind::NotFound => Reach::Missing,
        Err(_) => Reach::Refused,
    }
}

/// The number of a local display name, without any screen suffix (`:1`,
/// `unix:1`).
/// A name that carries a host is not local and is not probed.
fn local_number(display: &str) -> Option<&str> {
    let rest = display.strip_prefix("unix").unwrap_or(display);
    let rest = rest.strip_prefix(':')?;
    let number = rest.split('.').next()?;
    (!number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())).then_some(number)
}

#[cfg(target_os = "linux")]
fn abstract_connects(name: &str) -> bool {
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::SocketAddr;
    SocketAddr::from_abstract_name(name.as_bytes())
        .and_then(|address| UnixStream::connect_addr(&address))
        .is_ok()
}

#[cfg(not(target_os = "linux"))]
fn abstract_connects(_: &str) -> bool {
    false
}

impl Probe for SocketProbe {
    fn wayland(&self, socket: &Path) -> Reach {
        reach_of(UnixStream::connect(socket))
    }

    fn x11(&self, display: &str) -> Reach {
        let Some(number) = local_number(display) else {
            return Reach::Connected;
        };
        let path = format!("{X11_SOCKET_DIR}/X{number}");
        if abstract_connects(&path) {
            return Reach::Connected;
        }
        reach_of(UnixStream::connect(path))
    }
}

#[cfg(test)]
#[path = "display_probe_tests.rs"]
mod tests;
