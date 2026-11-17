//! Linux-specific socket boundary. OwnedFd closes partially initialized sockets.
use crate::{config::Result, interface::Interface};
use std::{
    io,
    mem::{size_of, zeroed},
    net::Ipv4Addr,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};
fn check(n: i32) -> io::Result<()> {
    if n < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn fd(kind: i32, protocol: i32) -> io::Result<OwnedFd> {
    let n = unsafe {
        libc::socket(
            libc::AF_INET,
            kind | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            protocol,
        )
    };
    check(n)?;
    Ok(unsafe { OwnedFd::from_raw_fd(n) })
}
fn set<T>(fd: &OwnedFd, level: i32, key: i32, value: &T) -> io::Result<()> {
    check(unsafe {
        libc::setsockopt(
            fd.as_raw_fd(),
            level,
            key,
            value as *const T as *const _,
            size_of::<T>() as _,
        )
    })
}
fn addr(ip: Ipv4Addr, port: u16) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_family: libc::AF_INET as _,
        sin_port: port.to_be(),
        sin_addr: libc::in_addr {
            s_addr: u32::from_ne_bytes(ip.octets()),
        },
        sin_zero: [0; 8],
    }
}
pub fn receiver(port: u16, group: Ipv4Addr, interfaces: &[(String, Interface)]) -> Result<OwnedFd> {
    let s = fd(libc::SOCK_DGRAM, 0)?;
    set(&s, libc::SOL_SOCKET, libc::SO_REUSEADDR, &1i32)?;
    set(&s, libc::SOL_SOCKET, libc::SO_RCVBUF, &262144i32)?;
    set(&s, libc::IPPROTO_IP, libc::IP_PKTINFO, &1i32)?;
    set(&s, libc::IPPROTO_IP, libc::IP_MULTICAST_ALL, &0i32)?;
    let a = addr(Ipv4Addr::UNSPECIFIED, port);
    check(unsafe {
        libc::bind(
            s.as_raw_fd(),
            &a as *const _ as *const _,
            size_of::<libc::sockaddr_in>() as _,
        )
    })?;
    if group.is_multicast() {
        for (_, i) in interfaces {
            let request = libc::ip_mreqn {
                imr_multiaddr: libc::in_addr {
                    s_addr: u32::from_ne_bytes(group.octets()),
                },
                imr_address: libc::in_addr {
                    s_addr: u32::from_ne_bytes(i.address.octets()),
                },
                imr_ifindex: i.index as _,
            };
            set(&s, libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP, &request)?;
        }
    }
    Ok(s)
}
/// Packet sockets bypass IP OUTPUT/POSTROUTING, so WAN masquerade cannot
/// overwrite the original discovery source. SOCK_DGRAM supplies Ethernet header.
pub fn sender(iface: &Interface) -> Result<OwnedFd> {
    if std::fs::read_to_string(format!("/sys/class/net/{}/type", iface.name))?.trim() != "1" {
        return Err("Source-preserving transmission requires an Ethernet/bridge interface".into());
    }
    let n = unsafe {
        libc::socket(
            libc::AF_PACKET,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0, // TX only: do not queue copies of unrelated IPv4 traffic.
        )
    };
    check(n).map_err(|e| format!("Packet socket needs CAP_NET_RAW: {e}"))?;
    Ok(unsafe { OwnedFd::from_raw_fd(n) })
}
pub struct Received {
    pub source: Ipv4Addr,
    pub sport: u16,
    pub destination: Ipv4Addr,
    pub ifindex: u32,
    pub size: usize,
    pub truncated: bool,
}
pub fn receive(s: &OwnedFd, data: &mut [u8]) -> io::Result<Received> {
    let mut source: libc::sockaddr_in = unsafe { zeroed() };
    // u64 alignment is sufficient for Linux cmsghdr on supported targets.
    let mut control = [0u64; 16];
    let mut iov = libc::iovec {
        iov_base: data.as_mut_ptr() as _,
        iov_len: data.len(),
    };
    let mut msg: libc::msghdr = unsafe { zeroed() };
    msg.msg_name = &mut source as *mut _ as _;
    msg.msg_namelen = size_of::<libc::sockaddr_in>() as _;
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = control.as_mut_ptr() as _;
    msg.msg_controllen = size_of_val(&control) as _;
    let n = unsafe { libc::recvmsg(s.as_raw_fd(), &mut msg, libc::MSG_DONTWAIT) };
    if n < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut info = None;
    unsafe {
        let mut c = libc::CMSG_FIRSTHDR(&msg);
        while !c.is_null() {
            if (*c).cmsg_level == libc::IPPROTO_IP
                && (*c).cmsg_type == libc::IP_PKTINFO
                && (*c).cmsg_len >= libc::CMSG_LEN(size_of::<libc::in_pktinfo>() as _) as _
            {
                info = Some(std::ptr::read_unaligned(
                    libc::CMSG_DATA(c) as *const libc::in_pktinfo
                ));
            }
            c = libc::CMSG_NXTHDR(&msg, c);
        }
    }
    let info = info.ok_or_else(|| io::Error::other("Missing IP_PKTINFO"))?;
    Ok(Received {
        source: Ipv4Addr::from(source.sin_addr.s_addr.to_ne_bytes()),
        sport: u16::from_be(source.sin_port),
        destination: Ipv4Addr::from(info.ipi_addr.s_addr.to_ne_bytes()),
        ifindex: info.ipi_ifindex as _,
        size: n as _,
        truncated: msg.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0,
    })
}
pub fn send(s: &OwnedFd, index: u32, dst: Ipv4Addr, packet: &[u8]) -> io::Result<()> {
    let mut a: libc::sockaddr_ll = unsafe { zeroed() };
    a.sll_family = libc::AF_PACKET as _;
    a.sll_protocol = (libc::ETH_P_IP as u16).to_be();
    a.sll_ifindex = index as _;
    a.sll_halen = 6;
    if dst.is_multicast() {
        let ip = dst.octets();
        a.sll_addr[..6].copy_from_slice(&[1, 0, 0x5e, ip[1] & 0x7f, ip[2], ip[3]]);
    } else {
        a.sll_addr[..6].fill(0xff);
    }
    let n = unsafe {
        libc::sendto(
            s.as_raw_fd(),
            packet.as_ptr() as _,
            packet.len(),
            0,
            &a as *const _ as _,
            size_of::<libc::sockaddr_ll>() as _,
        )
    };
    if n < 0 {
        return Err(io::Error::last_os_error());
    }
    if n as usize != packet.len() {
        return Err(io::Error::other("Partial packet send"));
    }
    Ok(())
}
