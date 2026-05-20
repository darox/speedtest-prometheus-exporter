# speedtest-prometheus-exporter

Minimal Prometheus exporter for Ookla Speedtest CLI.

## Quick Start

```bash
# Pre-built image
docker run -d -p 9798:9798 ghcr.io/darox/speedtest-prometheus-exporter:0.0.4
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

## Helm Chart

```bash
# Add repository
helm repo add speedtest-exporter https://darox.github.io/speedtest-prometheus-exporter
helm repo update

# Install
helm upgrade --install speedtest-exporter speedtest-exporter/speedtest-exporter

# With Prometheus Operator
helm upgrade --install speedtest-exporter speedtest-exporter/speedtest-exporter \
  --set serviceMonitor.enabled=true \
  --set 'serviceMonitor.labels.release=prometheus'

# Custom interval and server
helm upgrade --install speedtest-exporter speedtest-exporter/speedtest-exporter \
  --set env.SPEEDTEST_INTERVAL_SECS=60 \
  --set env.SPEEDTEST_SERVER_ID=1234
```

See [chart/README.md](chart/README.md) for all values.

## Architecture

- Rust binary statically linked with musl
- Ookla CLI subprocess for measurement
- `gcr.io/distroless/static` base image (no shell, no glibc)
test
