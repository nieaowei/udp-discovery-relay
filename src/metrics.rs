use serde::Serialize;
use std::{collections::VecDeque, path::Path, time::Instant};

pub fn remove_status(path: &Path) -> std::io::Result<()> {
    // procd's stop_service hook can already have removed the status file.
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}
#[derive(Serialize)]
pub struct Metrics {
    pub version: &'static str,
    pub pid: u32,
    pub uptime_seconds: u64,
    pub active_rules: usize,
    pub received: u64,
    pub forwarded: u64,
    pub dropped: u64,
    pub loop_filtered: u64,
    pub rss_bytes: u64,
    pub recent_errors: VecDeque<String>,
    pub interfaces: Vec<String>,
    #[serde(skip)]
    start: Option<Instant>,
    #[serde(skip)]
    last_error_log: Option<Instant>,
    #[serde(skip)]
    pub log_level: String,
    #[serde(skip)]
    suppressed_error_logs: u64,
}
impl Default for Metrics {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            pid: std::process::id(),
            uptime_seconds: 0,
            active_rules: 0,
            received: 0,
            forwarded: 0,
            dropped: 0,
            loop_filtered: 0,
            rss_bytes: 0,
            recent_errors: VecDeque::new(),
            interfaces: Vec::new(),
            start: Some(Instant::now()),
            last_error_log: None,
            log_level: "info".into(),
            suppressed_error_logs: 0,
        }
    }
}
impl Metrics {
    pub fn new(log_level: String) -> Self {
        Self {
            log_level,
            ..Default::default()
        }
    }
    pub fn error(&mut self, e: impl ToString) {
        self.error_for("error", e);
    }
    pub fn error_for(&mut self, rule_level: &str, e: impl ToString) {
        let e = e.to_string();
        let now = Instant::now();
        if self.log_level != "off"
            && rule_level != "off"
            && self
                .last_error_log
                .is_none_or(|t| now.duration_since(t).as_secs() >= 1)
        {
            eprintln!(
                "udp-discovery-relay: error: {e} ({} similar log entries suppressed)",
                self.suppressed_error_logs
            );
            self.last_error_log = Some(now);
            self.suppressed_error_logs = 0;
        } else {
            self.suppressed_error_logs += 1;
        }
        if self.recent_errors.len() == 16 {
            self.recent_errors.pop_front();
        }
        self.recent_errors.push_back(e.chars().take(512).collect());
    }
    pub fn publish(&mut self, path: &Path) -> std::io::Result<()> {
        self.uptime_seconds = self.start.unwrap().elapsed().as_secs();
        if let Ok(raw) = std::fs::read_to_string("/proc/self/status") {
            self.rss_bytes = raw
                .lines()
                .find_map(|l| {
                    l.strip_prefix("VmRSS:")
                        .and_then(|v| v.split_whitespace().next()?.parse::<u64>().ok())
                })
                .unwrap_or(0)
                * 1024;
        }
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, serde_json::to_vec(self)?)?;
        std::fs::rename(temp, path)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn status_cleanup_handles_an_already_removed_file() {
        let root = std::env::temp_dir().join(format!(
            "udr-status-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("status.json");
        std::fs::write(&path, "{}").unwrap();
        super::remove_status(&path).unwrap();
        super::remove_status(&path).unwrap();
        assert!(super::remove_status(&root).is_err());
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn errors_are_bounded() {
        let mut m = super::Metrics::default();
        for _ in 0..20 {
            m.error("failure");
        }
        assert_eq!(m.recent_errors.len(), 16);
    }
}
