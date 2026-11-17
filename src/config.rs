use serde::{Deserialize, Serialize};
use std::{collections::HashSet, net::Ipv4Addr, path::Path, process::Command};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn yes() -> bool {
    true
}
fn level() -> String {
    "info".into()
}
fn max_packet() -> usize {
    1400
}
fn bidirectional() -> String {
    "bidirectional".into()
}
fn udp() -> String {
    "udp".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "level")]
    pub log_level: String,
    #[serde(default = "yes")]
    pub loop_protection: bool,
    #[serde(default = "max_packet")]
    pub max_packet_size: usize,
    pub rules: Vec<Rule>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "udp")]
    pub protocol: String,
    pub port: u16,
    /// A multicast address or 255.255.255.255 (mapped to egress broadcast).
    pub multicast: Ipv4Addr,
    #[serde(default = "bidirectional")]
    pub direction: String,
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Vec<String>,
    /// Only for standalone Linux operation. OpenWrt uses logical networks.
    #[serde(default)]
    pub devices: Vec<String>,
    #[serde(default = "level")]
    pub log_level: String,
}
pub fn identifier(s: &str) -> bool {
    !s.is_empty() && s.len() <= 32 && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
pub fn device_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() < 16
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-.:".contains(&c))
        && s != "lo"
}
pub fn valid_level(s: &str) -> bool {
    matches!(s, "error" | "warn" | "info" | "debug" | "off")
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if !valid_level(&self.log_level)
            || !(64..=65507).contains(&self.max_packet_size)
            || self.rules.len() > 32
        {
            return Err("Invalid global options (max packet 64..65507, at most 32 rules)".into());
        }
        let mut ids = HashSet::new();
        let mut endpoints = HashSet::new();
        for r in &self.rules {
            if !identifier(&r.id)
                || !ids.insert(&r.id)
                || r.name.is_empty()
                || r.name.len() > 80
                || r.name.chars().any(char::is_control)
            {
                return Err("Invalid or duplicate rule id/name".into());
            }
            if r.protocol != "udp"
                || r.port == 0
                || matches!(r.port, 53 | 67 | 68 | 1900 | 5353)
                || !valid_level(&r.log_level)
            {
                return Err(
                    "Invalid UDP port/log level; DNS, DHCP, SSDP and mDNS are protected".into(),
                );
            }
            if !(r.multicast.is_multicast() || r.multicast == Ipv4Addr::BROADCAST)
                || r.multicast == Ipv4Addr::new(224, 0, 0, 251)
            {
                return Err("Expected IPv4 multicast or 255.255.255.255; mDNS is protected".into());
            }
            if !matches!(r.direction.as_str(), "bidirectional" | "oneway") {
                return Err("Direction must be bidirectional or oneway".into());
            }
            if !r.devices.is_empty()
                && (!r.network.is_empty() || !r.input.is_empty() || !r.output.is_empty())
            {
                return Err("Use either standalone devices or OpenWrt networks".into());
            }
            let names = r.names();
            let unique: HashSet<_> = names.iter().copied().collect();
            if unique.len() < 2
                || unique.len() > 16
                || r.devices.iter().any(|s| !device_name(s))
                || r.network
                    .iter()
                    .chain(&r.input)
                    .chain(&r.output)
                    .any(|s| !identifier(s))
            {
                return Err("Rules require 2..16 valid distinct interfaces/networks".into());
            }
            if r.direction == "oneway"
                && (r.input.is_empty()
                    || r.output.is_empty()
                    || !r.network.is_empty()
                    || !r.devices.is_empty()
                    || r.input.iter().any(|s| r.output.contains(s)))
            {
                return Err(
                    "One-way rules require disjoint input and output logical networks".into(),
                );
            }
            if r.direction == "bidirectional" && (!r.input.is_empty() || !r.output.is_empty()) {
                return Err("Bidirectional rules use network or devices".into());
            }
            if r.enabled && !endpoints.insert((r.port, r.multicast)) {
                return Err("Duplicate enabled destination/port".into());
            }
        }
        Ok(())
    }
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let raw = if let Some(p) = path {
            if std::fs::metadata(p)?.len() > 65536 {
                return Err("Configuration exceeds 64 KiB".into());
            }
            std::fs::read(p)?
        } else {
            let out = Command::new("/usr/libexec/udp-discovery-relay-config").output()?;
            if !out.status.success() {
                return Err(String::from_utf8_lossy(&out.stderr).to_string().into());
            }
            out.stdout
        };
        if raw.len() > 65536 {
            return Err("Configuration exceeds 64 KiB".into());
        }
        let value: Self = serde_json::from_slice(&raw)?;
        value.validate()?;
        Ok(value)
    }
}
impl Rule {
    pub fn names(&self) -> Vec<&str> {
        self.network
            .iter()
            .chain(&self.input)
            .chain(&self.output)
            .chain(&self.devices)
            .map(String::as_str)
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_invalid_and_unknown() {
        let mut c: Config = serde_json::from_str(include_str!("../tests/config.json")).unwrap();
        assert!(c.validate().is_ok());
        c.rules[0].port = 5353;
        assert!(c.validate().is_err());
        c.rules[0].port = 53317;
        c.rules[0].devices[0] = "a;reboot".into();
        assert!(c.validate().is_err());
        assert!(serde_json::from_str::<Config>(r#"{"rules":[],"oops":1}"#).is_err());
    }
}
