use crate::{
    config::{Config, Result, Rule},
    filter::LoopFilter,
    interface::{self, Interface},
    metrics::Metrics,
    packet, socket,
};
use std::{
    io,
    os::fd::{AsRawFd, OwnedFd},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
struct Active {
    rule: Rule,
    interfaces: Vec<(String, Interface)>,
    recv: OwnedFd,
    send: Vec<OwnedFd>,
}
fn construct(c: &Config, all: &[Interface], m: &mut Metrics) -> Vec<Active> {
    let mut active = Vec::new();
    if !c.enabled {
        m.active_rules = 0;
        m.interfaces.clear();
        return active;
    }
    for rule in c.rules.iter().filter(|r| r.enabled) {
        let result = (|| -> Result<Active> {
            let interfaces = interface::resolve(rule, all)?;
            let recv = socket::receiver(rule.port, rule.multicast, &interfaces)?;
            let send = interfaces
                .iter()
                .map(|(_, i)| socket::sender(i))
                .collect::<Result<Vec<_>>>()?;
            Ok(Active {
                rule: rule.clone(),
                interfaces,
                recv,
                send,
            })
        })();
        match result {
            Ok(a) => active.push(a),
            Err(e) => m.error_for(&rule.log_level, format!("{}: {e}", rule.id)),
        }
    }
    m.active_rules = active.len();
    m.interfaces = active
        .iter()
        .flat_map(|a| {
            a.interfaces
                .iter()
                .map(|(n, i)| format!("{n}: {} {}", i.name, i.address))
        })
        .collect();
    active
}
fn enabled_log(global: &str, rule: &str, level: &str) -> bool {
    fn rank(s: &str) -> u8 {
        match s {
            "off" => 0,
            "error" => 1,
            "warn" => 2,
            "info" => 3,
            _ => 4,
        }
    }
    rank(global) >= rank(level) && rank(rule) >= rank(level)
}
pub fn run(path: Option<&Path>, status: &Path) -> Result<()> {
    let stop = Arc::new(AtomicBool::new(false));
    let reload = Arc::new(AtomicBool::new(false));
    for sig in [libc::SIGTERM, libc::SIGINT] {
        signal_hook::flag::register(sig, stop.clone())?;
    }
    signal_hook::flag::register(libc::SIGHUP, reload.clone())?;
    let mut c = Config::load(path)?;
    let mut m = Metrics::new(c.log_level.clone());
    let mut all = interface::all()?;
    let mut active = construct(&c, &all, &mut m);
    crate::privilege::restrict()?;
    let mut filter = LoopFilter::default();
    let mut receive = vec![0u8; 65508];
    let mut wire = Vec::with_capacity(65535);
    let mut maintenance = Instant::now();
    let mut publish = Instant::now();
    m.publish(status)?;
    if enabled_log(&c.log_level, "info", "info") {
        eprintln!(
            "udp-discovery-relay: started {} ({} active rules)",
            env!("CARGO_PKG_VERSION"),
            active.len()
        );
    }
    let mut polls = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        let mut rebuild = false;
        if reload.swap(false, Ordering::Relaxed) {
            match Config::load(path) {
                Ok(next) => {
                    c = next;
                    m.log_level = c.log_level.clone();
                    rebuild = true;
                    if enabled_log(&c.log_level, "info", "info") {
                        eprintln!("udp-discovery-relay: configuration reloaded");
                    }
                }
                Err(e) => m.error(format!(
                    "Reload rejected; keeping previous configuration: {e}"
                )),
            }
        }
        if maintenance.elapsed() >= Duration::from_secs(5) {
            maintenance = Instant::now();
            match interface::all() {
                Ok(next) => {
                    rebuild |= next != all;
                    all = next;
                }
                Err(e) => m.error(e),
            }
            // Resolve netifd again even when Linux device addresses are unchanged.
            for a in &active {
                match interface::resolve(&a.rule, &all) {
                    Ok(i) => rebuild |= i != a.interfaces,
                    Err(_) => rebuild = true,
                }
            }
            rebuild |= active.len()
                != if c.enabled {
                    c.rules.iter().filter(|r| r.enabled).count()
                } else {
                    0
                };
        }
        if rebuild {
            active.clear();
            active = construct(&c, &all, &mut m);
        }
        polls.clear();
        polls.extend(active.iter().map(|a| libc::pollfd {
            fd: a.recv.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        }));
        let ready = unsafe { libc::poll(polls.as_mut_ptr(), polls.len() as _, 1000) };
        if ready < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
            return Err(io::Error::last_os_error().into());
        }
        for (a, p) in active.iter().zip(&polls) {
            if p.revents & (libc::POLLERR | libc::POLLNVAL | libc::POLLHUP) != 0 {
                m.error(format!("{}: socket changed; rebuilding", a.rule.id));
                reload.store(true, Ordering::Relaxed);
            }
            if p.revents & libc::POLLIN == 0 {
                continue;
            }
            // Bound each batch so a noisy rule cannot starve signals/other rules.
            for _ in 0..64 {
                let r = match socket::receive(&a.recv, &mut receive) {
                    Ok(r) => r,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(e) => {
                        m.error(e);
                        break;
                    }
                };
                m.received += 1;
                let Some((ingress_name, ingress)) =
                    a.interfaces.iter().find(|(_, i)| i.index == r.ifindex)
                else {
                    m.dropped += 1;
                    continue;
                };
                let valid_dest = if a.rule.multicast.is_multicast() {
                    r.destination == a.rule.multicast
                } else {
                    r.destination == std::net::Ipv4Addr::BROADCAST
                        || r.destination == ingress.broadcast()
                };
                if r.truncated
                    || r.size > c.max_packet_size
                    || !valid_dest
                    || r.sport == 0
                    || r.source.is_unspecified()
                    || r.source.is_multicast()
                    || r.source.is_loopback()
                    || r.source == ingress.broadcast()
                    || !ingress.contains(r.source)
                    || all.iter().any(|i| i.address == r.source)
                    || a.rule.direction == "oneway" && !a.rule.input.contains(ingress_name)
                {
                    m.dropped += 1;
                    continue;
                }
                let data = &receive[..r.size];
                // Broadcast addresses differ between subnets; normalize cache key.
                if c.loop_protection
                    && filter.duplicate(
                        r.source,
                        r.sport,
                        a.rule.port,
                        a.rule.multicast,
                        data,
                        Instant::now(),
                    )
                {
                    m.loop_filtered += 1;
                    continue;
                }
                for (index, (name, egress)) in a.interfaces.iter().enumerate() {
                    if egress.index == ingress.index
                        || a.rule.direction == "oneway" && !a.rule.output.contains(name)
                    {
                        continue;
                    }
                    if data.len() + 28 > egress.mtu {
                        m.dropped += 1;
                        m.error_for(
                            &a.rule.log_level,
                            format!(
                                "{}: datagram exceeds {} MTU {}",
                                a.rule.id, egress.name, egress.mtu
                            ),
                        );
                        continue;
                    }
                    let dst = if a.rule.multicast.is_multicast() {
                        a.rule.multicast
                    } else {
                        egress.broadcast()
                    };
                    packet::build(&mut wire, r.source, dst, r.sport, a.rule.port, data);
                    match socket::send(&a.send[index], egress.index, dst, &wire) {
                        Ok(()) => {
                            m.forwarded += 1;
                            if enabled_log(&c.log_level, &a.rule.log_level, "debug") {
                                eprintln!(
                                    "udp-discovery-relay: {} {}:{} {} -> {}",
                                    a.rule.id, r.source, r.sport, ingress.name, egress.name
                                );
                            }
                        }
                        Err(e) => {
                            m.dropped += 1;
                            m.error_for(
                                &a.rule.log_level,
                                format!("{} via {}: {e}", a.rule.id, egress.name),
                            );
                        }
                    }
                }
            }
        }
        if publish.elapsed() >= Duration::from_secs(5) {
            if let Err(e) = m.publish(status) {
                eprintln!("udp-discovery-relay: status write: {e}");
            }
            publish = Instant::now();
        }
    }
    drop(active);
    std::fs::remove_file(status)?;
    if enabled_log(&c.log_level, "info", "info") {
        eprintln!("udp-discovery-relay: stopped");
    }
    Ok(())
}
