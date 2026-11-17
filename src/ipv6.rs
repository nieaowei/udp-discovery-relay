//! IPv6 is isolated from the IPv4 relay. No IPv6 socket or multicast membership
//! is created. IPv6 source scopes and hop-limit rules require a separate design.
pub fn supported() -> bool {
    false
}
#[cfg(test)]
mod tests {
    #[test]
    fn remains_explicitly_disabled() {
        assert!(!super::supported());
    }
}
