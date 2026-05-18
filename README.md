# speedtest-prometheus-exporter

Minimal Prometheus exporter for Ookla Speedtest CLI.

## Quick Start

```bash
docker run -d -p 9798:9798 speedtest-exporter
```

## Configuration

| Environment Variable | Default | Description |
|---|---|---|
| `PORT` | `9798` | HTTP listener port |
| `SPEEDTEST_INTERVAL_SECS` | `300` | Seconds between speedtests (min: 30) |
| `SPEEDTEST_SERVER_ID` | *(auto)* | Pin to specific Ookla server |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |

## Metrics

| Metric | Type | Description |
|---|---|---|
| `speedtest_download_bps` | Gauge | Download speed (bits/s) |
| `speedtest_upload_bps` | Gauge | Upload speed (bits/s) |
| `speedtest_ping_latency_seconds` | Gauge | Average ping latency |
| `speedtest_jitter_seconds` | Gauge | Ping jitter |
| `speedtest_packet_loss_ratio` | Gauge | Packet loss (0.0–1.0) |
| `speedtest_duration_seconds` | Gauge | Test duration |
| `speedtest_last_success` | Gauge | 1 = success, 0 = failure |
| `speedtest_server_info{server, country, isp}` | Gauge | Server info labels |

## Building

```bash
# Single arch
docker build -t speedtest-exporter .

# Multi-arch (arm64 + amd64)
docker buildx build --platform linux/arm64,linux/amd64 -t speedtest-exporter .

# Run tests
docker build --target test -t speedtest-exporter:test .
```

## Architecture

- Rust binary statically linked with musl
- Ookla CLI v1.2.0 subprocess for measurement
- `gcr.io/distroless/static` base image (no shell, no glibc)
- 8.3MB image, 0 CVEs
