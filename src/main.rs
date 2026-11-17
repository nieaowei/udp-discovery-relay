use std::path::PathBuf;
fn main() {
    if let Err(e) = execute() {
        eprintln!("udp-discovery-relay: {e}");
        std::process::exit(1);
    }
}
fn execute() -> udp_discovery_relay::config::Result<()> {
    let mut config = None;
    let mut check = false;
    let mut status = PathBuf::from("/var/run/udp-discovery-relay/status.json");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                config = Some(PathBuf::from(
                    args.next().ok_or("--config requires a path")?,
                ))
            }
            "--status" => status = PathBuf::from(args.next().ok_or("--status requires a path")?),
            "--check" => check = true,
            "--uci" => {}
            "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" => {
                println!(
                    "udp-discovery-relay [--uci | --config FILE.json] [--check] [--status FILE]\nLinux IPv4 relay; requires CAP_NET_RAW. IPv6 is not enabled."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    if check {
        udp_discovery_relay::config::Config::load(config.as_deref())?;
        println!("Configuration valid");
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        udp_discovery_relay::relay::run(config.as_deref(), &status)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = status;
        Err("The relay requires Linux; --check and unit tests work on this host".into())
    }
}
