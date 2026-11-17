use std::{
    collections::{HashMap, VecDeque},
    net::Ipv4Addr,
    time::{Duration, Instant},
};
/// Bounded keyed hashes make payload storage unnecessary. A retransmission within
/// 750 ms is intentionally coalesced; repeated announcements later still pass.
pub struct LoopFilter {
    map: HashMap<u64, Instant>,
    order: VecDeque<(u64, Instant)>,
    seed: std::collections::hash_map::RandomState,
}
impl Default for LoopFilter {
    fn default() -> Self {
        Self {
            map: HashMap::with_capacity(4096),
            order: VecDeque::with_capacity(4096),
            seed: Default::default(),
        }
    }
}
impl LoopFilter {
    pub fn duplicate(
        &mut self,
        src: Ipv4Addr,
        sport: u16,
        port: u16,
        destination: Ipv4Addr,
        data: &[u8],
        now: Instant,
    ) -> bool {
        use std::hash::BuildHasher;
        while self
            .order
            .front()
            .is_some_and(|(_, t)| now.duration_since(*t) >= Duration::from_millis(750))
        {
            let (k, t) = self.order.pop_front().unwrap();
            if self.map.get(&k) == Some(&t) {
                self.map.remove(&k);
            }
        }
        let key = self.seed.hash_one((src, sport, port, destination, data));
        if self.map.contains_key(&key) {
            return true;
        }
        if self.order.len() >= 4096 {
            let (k, _) = self.order.pop_front().unwrap();
            self.map.remove(&k);
        }
        self.map.insert(key, now);
        self.order.push_back((key, now));
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_window() {
        let mut f = LoopFilter::default();
        let t = Instant::now();
        let s = Ipv4Addr::LOCALHOST;
        assert!(!f.duplicate(s, 1, 2, s, b"a", t));
        assert!(f.duplicate(s, 1, 2, s, b"a", t));
        assert!(!f.duplicate(s, 1, 2, s, b"a", t + Duration::from_secs(1)));
        for i in 0..10000u32 {
            f.duplicate(s, 1, 2, s, &i.to_be_bytes(), t + Duration::from_secs(2));
        }
        assert!(f.map.len() <= 4096);
        assert!(f.order.len() <= 4096);
    }
}
