# badpipes

A simple lightweight "network connectivity monitor". Polls TCP connectivity to one or more hosts and logs timestamped connect/disconnect events. Only prints when the state changes, so you get a clean log of outages rather than a wall of noise. I made this mainly to watch for 
my internet dropping at home. 

## Install

Download a prebuilt binary from the [releases page](../../releases) or build from source:

```bash
cargo install --path .
```

**macOS**: If you get "cannot be opened because the developer cannot be verified", run:
```bash
xattr -d com.apple.quarantine badpipes
```

## Usage

```bash
badpipes [OPTIONS]
```

### Options

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--interval` | `-i` | Poll interval in seconds | `5` |
| `--log-file` | `-l` | Log file path (append mode) | None (stdout only) |
| `--target` | `-t` | Target address (`ip:port`), can be repeated | `8.8.8.8:53`, `1.1.1.1:53`, `8.8.4.4:53` |
| `--once` | `-o` | Check once and exit (exit code 0 = connected, 1 = not) | Off |
| `--timeout` | | Connection timeout in seconds | `3` |
| `--count` | `-c` | Poll N times then exit | Unlimited |
| `--quiet` | `-q` | Suppress stdout (only write to log file) | Off |
| `--json` | `-j` | Output in JSON format (NDJSON) | Off |

### Examples

```bash
# Default: poll 3 DNS servers every 5s, print to stdout
badpipes

# Poll every 10 seconds and log to a file
badpipes -i 10 -l connectivity.log

# Monitor a specific host
badpipes -t 192.168.1.1:80

# Monitor multiple custom targets
badpipes -t 10.0.0.1:53 -t 10.0.0.2:53

# One-shot check for scripting
badpipes --once && echo "online" || echo "offline"

# Shorter timeout for fast failure
badpipes --timeout 1

# Run 10 polls then exit
badpipes -c 10

# Silent background logging
badpipes -q -l connectivity.log

# JSON output for piping into jq or other tools
badpipes --once --json

# Stream JSON to a file
badpipes -j -l connectivity.json
```

### Output

```
[2026-03-11 14:32:01] Connected — en0 (Wi-Fi)
[2026-03-11 14:35:47] Not connected — en0 (Wi-Fi)
[2026-03-11 14:36:12] Connected — en0 (Wi-Fi), en8 (Ethernet)
```

With `--json`:
```json
{"timestamp":"2026-03-11T14:32:01","status":"connected","interfaces":[{"name":"en0","type":"Wi-Fi"}]}
{"timestamp":"2026-03-11T14:35:47","status":"not_connected","interfaces":[{"name":"en0","type":"Wi-Fi"}]}
{"timestamp":"2026-03-11T14:36:12","status":"connected","interfaces":[{"name":"en0","type":"Wi-Fi"},{"name":"en8","type":"Ethernet"}]}
```

## How it works

badpipes attempts a TCP connection (with a configurable timeout, default 3s) to each target host. If **any** target is reachable, the status is "Connected". It only logs when the state transitions between connected and disconnected, making it easy to spot outages at a glance.

Each log entry includes the active network interfaces and their detected type (Wi-Fi, Ethernet, VPN/Tunnel, etc.). A new entry is also logged when interfaces change without a connectivity state change — for example, switching from Wi-Fi to Ethernet.