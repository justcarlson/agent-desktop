#![allow(dead_code)]

use agent_desktop_core::{AdapterError, Deadline, ProcessIdentity};
use std::mem::size_of;

#[path = "tcp_listener_metadata.rs"]
mod metadata;
use metadata::SocketMetadata;

const PROC_PIDFDSOCKETINFO: libc::c_int = 3;
const SOCKINFO_TCP: libc::c_int = 2;
const TSI_S_LISTEN: libc::c_int = 1;

#[repr(C)]
#[derive(Clone, Copy)]
struct ProcFileInfo {
    open_flags: u32,
    status: u32,
    offset: i64,
    file_type: i32,
    guard_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ProcFdInfo {
    fd: i32,
    fd_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SockBufInfo {
    values: [u32; 5],
    flags: i16,
    timeout: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InSockInfo {
    foreign_port: i32,
    local_port: i32,
    generation: u64,
    flags: u32,
    flow: u32,
    address_flags: u8,
    ttl: u8,
    reserved: u32,
    addresses: [[u64; 2]; 2],
    ipv4_tos: u8,
    ipv6: InSockIpv6,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InSockIpv6 {
    hop_limit: u8,
    checksum: i32,
    interface_index: u16,
    hops: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TcpSockInfo {
    network: InSockInfo,
    state: i32,
    timers: [i32; 4],
    mss: i32,
    flags: u32,
    reserved: u32,
    tcp_control_block: u64,
}

#[repr(C)]
union SocketProtocolInfo {
    tcp: TcpSockInfo,
    reserved: [u64; 66],
}

#[repr(C)]
struct SocketInfo {
    stat: libc::vinfo_stat,
    metadata: SocketMetadata,
    receive: SockBufInfo,
    send: SockBufInfo,
    kind: i32,
    reserved: u32,
    protocol_info: SocketProtocolInfo,
}

#[repr(C)]
struct SocketFdInfo {
    file: ProcFileInfo,
    socket: SocketInfo,
}

pub(crate) fn owns_listener(
    process: &ProcessIdentity,
    port: u16,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let pid = i32::try_from(process.pid.get())
        .map_err(|_| AdapterError::internal("Process id exceeds the macOS pid range"))?;
    if !crate::system::process::process_identity::matches_instance(pid, &process.instance)? {
        return Ok(false);
    }
    let mut fds = vec![ProcFdInfo { fd: 0, fd_type: 0 }; 8192];
    let returned = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            fds.as_mut_ptr().cast(),
            (fds.len() * size_of::<ProcFdInfo>()) as i32,
        )
    };
    if returned < 0 || returned as usize >= fds.len() * size_of::<ProcFdInfo>() {
        return Err(AdapterError::internal(
            "macOS could not read the process descriptor list",
        ));
    }
    fds.truncate(returned as usize / size_of::<ProcFdInfo>());
    for fd in fds {
        if deadline.is_expired() {
            return Err(AdapterError::timeout(
                "Checking the DevTools listener owner",
            ));
        }
        let mut info = std::mem::MaybeUninit::<SocketFdInfo>::zeroed();
        let returned = unsafe {
            libc::proc_pidfdinfo(
                pid,
                fd.fd,
                PROC_PIDFDSOCKETINFO,
                info.as_mut_ptr().cast(),
                size_of::<SocketFdInfo>() as i32,
            )
        };
        let minimum = std::mem::offset_of!(SocketFdInfo, socket)
            + std::mem::offset_of!(SocketInfo, protocol_info)
            + size_of::<TcpSockInfo>();
        if returned < minimum as i32 {
            continue;
        }
        let info = unsafe { info.assume_init() };
        if info.socket.kind != SOCKINFO_TCP {
            continue;
        }
        let tcp = unsafe { info.socket.protocol_info.tcp };
        if tcp.state == TSI_S_LISTEN
            && u16::from_be(tcp.network.local_port as u16) == port
            && loopback_ipv4(&tcp.network)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn loopback_ipv4(network: &InSockInfo) -> bool {
    if network.address_flags & 1 == 0 {
        return false;
    }
    let local = network.addresses[1][1].to_ne_bytes();
    u32::from_be_bytes([local[4], local[5], local[6], local[7]]) == 0x7f00_0001
}

#[cfg(test)]
#[path = "tcp_listener_tests.rs"]
mod tests;
