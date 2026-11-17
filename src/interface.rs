use crate::config::{Result, Rule, device_name};
use std::{
    ffi::{CStr, CString},
    net::Ipv4Addr,
    process::Command,
};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Interface {
    pub name: String,
    pub index: u32,
    pub address: Ipv4Addr,
    pub mask: Ipv4Addr,
    pub mtu: usize,
}
impl Interface {
    pub fn contains(&self, address: Ipv4Addr) -> bool {
        u32::from(address) & u32::from(self.mask) == u32::from(self.address) & u32::from(self.mask)
    }
    pub fn broadcast(&self) -> Ipv4Addr {
        Ipv4Addr::from(u32::from(self.address) | !u32::from(self.mask))
    }
}
pub fn all() -> Result<Vec<Interface>> {
    let mut ptr = std::ptr::null_mut();
    // SAFETY: getifaddrs initializes a linked list; guard frees it on every path.
    if unsafe { libc::getifaddrs(&mut ptr) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    struct Guard(*mut libc::ifaddrs);
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe { libc::freeifaddrs(self.0) }
        }
    }
    let _guard = Guard(ptr);
    let mut p = ptr;
    let mut out = Vec::new();
    while !p.is_null() {
        // SAFETY: pointers and names belong to the live getifaddrs list.
        let item = unsafe { &*p };
        p = item.ifa_next;
        if item.ifa_addr.is_null()
            || item.ifa_netmask.is_null()
            || unsafe { (*item.ifa_addr).sa_family } as i32 != libc::AF_INET
            || item.ifa_flags & libc::IFF_UP as u32 == 0
        {
            continue;
        }
        let name = unsafe { CStr::from_ptr(item.ifa_name) }
            .to_string_lossy()
            .to_string();
        let index = unsafe { libc::if_nametoindex(item.ifa_name) };
        let address = Ipv4Addr::from(
            unsafe {
                (*(item.ifa_addr as *const libc::sockaddr_in))
                    .sin_addr
                    .s_addr
            }
            .to_ne_bytes(),
        );
        let mask = Ipv4Addr::from(
            unsafe {
                (*(item.ifa_netmask as *const libc::sockaddr_in))
                    .sin_addr
                    .s_addr
            }
            .to_ne_bytes(),
        );
        let mtu = std::fs::read_to_string(format!("/sys/class/net/{name}/mtu"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1500);
        out.push(Interface {
            name,
            index,
            address,
            mask,
            mtu,
        });
    }
    out.sort();
    Ok(out)
}
pub fn resolve(rule: &Rule, available: &[Interface]) -> Result<Vec<(String, Interface)>> {
    let mut result = Vec::new();
    for name in rule.names() {
        let device = if !rule.devices.is_empty() {
            name.to_string()
        } else {
            let out = Command::new("ubus")
                .args(["call", &format!("network.interface.{name}"), "status"])
                .output()?;
            if !out.status.success() {
                return Err(format!("Logical network {name} is unavailable").into());
            }
            let v: serde_json::Value = serde_json::from_slice(&out.stdout)?;
            if v["up"] != true {
                return Err(format!("Logical network {name} is down").into());
            }
            v["l3_device"]
                .as_str()
                .ok_or("Missing netifd l3_device")?
                .to_string()
        };
        if !device_name(&device) {
            return Err("Unsupported device name".into());
        }
        let iface = available
            .iter()
            .find(|i| i.name == device)
            .ok_or_else(|| format!("{device} has no active IPv4 address"))?
            .clone();
        if result.iter().any(|(_, i): &(String, Interface)| {
            i.index == iface.index || i.contains(iface.address) || iface.contains(i.address)
        }) {
            return Err("Interfaces must be distinct, non-overlapping IPv4 subnets".into());
        }
        if u32::from(iface.mask).count_ones() > 30 {
            return Err("Point-to-point and /31-/32 interfaces are unsupported".into());
        }
        result.push((name.into(), iface));
    }
    Ok(result)
}
pub fn c_name(name: &str) -> Result<CString> {
    Ok(CString::new(name)?)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subnet_math() {
        let i = Interface {
            name: "a".into(),
            index: 1,
            address: "192.168.8.1".parse().unwrap(),
            mask: "255.255.255.0".parse().unwrap(),
            mtu: 1500,
        };
        assert!(i.contains("192.168.8.9".parse().unwrap()));
        assert!(!i.contains("192.168.1.9".parse().unwrap()));
        assert_eq!(i.broadcast().to_string(), "192.168.8.255");
    }
}
