use std::fs::OpenOptions;
use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
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
}

const DEFAULT_TARGETS: &[&str] = &["8.8.8.8:53", "1.1.1.1:53", "8.8.4.4:53"];

fn is_connected(target: &str) -> bool {
    TcpStream::connect_timeout(&target.parse().unwrap(), Duration::from_secs(3)).is_ok()
}

fn log_event(message: &str, log_file: &Option<PathBuf>) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{timestamp}] {message}");
    println!("{line}");

    if let Some(path) = log_file {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn main() {
    let args = Args::parse();
    let targets: Vec<&str> = if args.target.is_empty() {
        DEFAULT_TARGETS.to_vec()
    } else {
        args.target.iter().map(|s| s.as_str()).collect()
    };
    let mut previous_state: Option<bool> = None;

    loop {
        let connected = targets.iter().any(|host| is_connected(host));

        if previous_state != Some(connected) {
            let message = if connected {
                "Connected"
            } else {
                "Not connected"
            };
            log_event(message, &args.log_file);
            previous_state = Some(connected);
        }

        sleep(Duration::from_secs(args.interval));
    }
}
