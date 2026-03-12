use std::collections::HashSet;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process;
use std::thread::sleep;
use std::time::Duration;

use chrono::Local;
use clap::Parser;

#[derive(Parser)]
#[command(name = "badpipes", about = "Network connectivity monitor")]
struct Args {
    /// Poll interval in seconds
    #[arg(short, long, default_value_t = 5)]
    interval: u64,

    /// Optional log file path (append mode)
    #[arg(short, long)]
    log_file: Option<PathBuf>,

    /// Target addresses to monitor (ip:port). Can be specified multiple times.
    #[arg(short, long)]
    target: Vec<String>,

    /// Check once and exit (0 = connected, 1 = not connected)
    #[arg(short, long)]
    once: bool,

    /// Connection timeout in seconds
    #[arg(long, default_value_t = 3)]
    timeout: u64,

    /// Poll N times then exit
    #[arg(short, long)]
    count: Option<u64>,

    /// Suppress stdout (only write to log file)
    #[arg(short, long)]
    quiet: bool,

    /// Output in JSON format (NDJSON)
    #[arg(short, long)]
    json: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Connected,
    NotConnected,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Connected => write!(f, "Connected"),
            Status::NotConnected => write!(f, "Not connected"),
        }
    }
}

impl Status {
    fn as_json_str(self) -> &'static str {
        match self {
            Status::Connected => "connected",
            Status::NotConnected => "not_connected",
        }
    }
}

const DEFAULT_TARGETS: &[&str] = &["8.8.8.8:53", "1.1.1.1:53", "8.8.4.4:53"];

fn is_connected(target: &str, timeout: Duration) -> bool {
    let Ok(addr) = target.parse() else {
        eprintln!("warning: invalid target address: {target}");
        return false;
    };
    TcpStream::connect_timeout(&addr, timeout).is_ok()
}

/// Returns the set of active non-loopback interface names.
fn get_active_interfaces() -> HashSet<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .map(|iface| iface.name)
        .collect()
}

/// Best-effort guess at interface type from its name.
fn guess_interface_type(name: &str) -> &'static str {
    let lower = name.to_lowercase();

    // Windows uses descriptive names
    if lower.contains("wi-fi") || lower.contains("wifi") {
        return "Wi-Fi";
    }
    if lower.contains("ethernet") {
        return "Ethernet";
    }

    // Linux wireless
    if lower.starts_with("wlan")
        || lower.starts_with("wlp")
        || lower.starts_with("wlo")
    {
        return "Wi-Fi";
    }

    // Linux ethernet
    if lower.starts_with("eth")
        || lower.starts_with("enp")
        || lower.starts_with("eno")
        || lower.starts_with("ens")
    {
        return "Ethernet";
    }

    // macOS: en0 is typically Wi-Fi on modern Macs
    if lower == "en0" {
        return "Wi-Fi";
    }

    // macOS: other en* are typically Ethernet/Thunderbolt
    if lower.starts_with("en") && lower[2..].chars().all(|c| c.is_ascii_digit()) {
        return "Ethernet";
    }

    // VPN / tunnel interfaces
    if lower.starts_with("utun")
        || lower.starts_with("tun")
        || lower.starts_with("tap")
    {
        return "VPN/Tunnel";
    }

    // Docker / container interfaces
    if lower.starts_with("docker")
        || lower.starts_with("br-")
        || lower.starts_with("veth")
    {
        return "Docker";
    }

    "Unknown"
}

/// Escape a string for safe inclusion in a JSON string value.
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn log_event(
    status: Status,
    interfaces: &[String],
    log_file: &Option<PathBuf>,
    quiet: bool,
    json: bool,
) {
    let line = if json {
        let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S");
        let ifaces_json: Vec<String> = interfaces
            .iter()
            .map(|name| {
                format!(
                    "{{\"name\":\"{}\",\"type\":\"{}\"}}",
                    json_escape(name),
                    guess_interface_type(name),
                )
            })
            .collect();
        format!(
            "{{\"timestamp\":\"{timestamp}\",\"status\":\"{}\",\"interfaces\":[{}]}}",
            status.as_json_str(),
            ifaces_json.join(","),
        )
    } else {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        if interfaces.is_empty() {
            format!("[{timestamp}] {status}")
        } else {
            let iface_list: Vec<String> = interfaces
                .iter()
                .map(|n| format!("{} ({})", n, guess_interface_type(n)))
                .collect();
            format!("[{timestamp}] {status} — {}", iface_list.join(", "))
        }
    };

    if !quiet {
        println!("{line}");
    }

    if let Some(path) = log_file {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

/// Collect interfaces into a sorted Vec for deterministic output.
fn sorted_interfaces(set: &HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = set.iter().cloned().collect();
    v.sort();
    v
}

fn main() {
    let args = Args::parse();
    let targets: Vec<&str> = if args.target.is_empty() {
        DEFAULT_TARGETS.to_vec()
    } else {
        args.target.iter().map(|s| s.as_str()).collect()
    };
    let timeout = Duration::from_secs(args.timeout);

    if args.once {
        let connected = targets.iter().any(|host| is_connected(host, timeout));
        let status = if connected { Status::Connected } else { Status::NotConnected };
        let interfaces = get_active_interfaces();
        log_event(status, &sorted_interfaces(&interfaces), &args.log_file, args.quiet, args.json);
        process::exit(if connected { 0 } else { 1 });
    }

    let mut previous_state: Option<bool> = None;
    let mut previous_interfaces = HashSet::new();
    let mut polls: u64 = 0;

    loop {
        let connected = targets.iter().any(|host| is_connected(host, timeout));
        let current_interfaces = get_active_interfaces();

        let state_changed = previous_state != Some(connected);
        let interfaces_changed = current_interfaces != previous_interfaces;

        if state_changed || interfaces_changed {
            let status = if connected { Status::Connected } else { Status::NotConnected };
            log_event(status, &sorted_interfaces(&current_interfaces), &args.log_file, args.quiet, args.json);
            previous_state = Some(connected);
            previous_interfaces = current_interfaces;
        }

        polls += 1;
        if let Some(count) = args.count {
            if polls >= count {
                break;
            }
        }

        sleep(Duration::from_secs(args.interval));
    }
}