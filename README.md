# badpipes

A simple lightweight "network connectivity monitor". Polls TCP connectivity to one or more hosts and logs timestamped connect/disconnect events. Only prints when the state changes, so you get a clean log of outages rather than a wall of noise. I made this mainly to watch for 
my internet dropping at home. 

## Install

```bash
cargo install --path .
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
```

### Output

```
[2026-03-11 14:32:01] Connected
[2026-03-11 14:35:47] Not connected
[2026-03-11 14:36:12] Connected
```

## How it works

badpipes attempts a TCP connection (with a 3-second timeout) to each target host. If **any** target is reachable, the status is "Connected". It only logs when the state transitions between connected and disconnected, making it easy to spot outages at a glance.