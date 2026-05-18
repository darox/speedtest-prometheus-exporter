# speedtest-prometheus-exporter

Minimal Prometheus exporter that runs Ookla Speedtest CLI in the background and exposes metrics.

## Tech Stack

- Rust 2024 edition, resolver 3
- axum 0.8.9, tokio 1.52.3, prometheus 0.14.0
- Statically linked musl binary on `distroless/static` (no shell, no glibc)
- Ookla CLI v1.2.0 (statically linked subprocess)

## Architecture

```
main.rs          — Wiring: config init, HTTP routes, run_loop
runner.rs        — SpeedtestRunner trait + OoklaCliRunner adapter + SpeedtestResult domain type
metrics.rs       — MetricsUpdater trait + PrometheusMetrics adapter + RecordingMetrics test adapter
config.rs        — Config from env with validation (min 30s interval)
logging.rs       — tracing init
```

Two real seams:
- `SpeedtestRunner` — swap Ookla CLI for mock or alternative speedtest tool
- `MetricsUpdater` — swap Prometheus for OTLP, InfluxDB, etc.

## Build & Test

```bash
# Build
docker build -t speedtest-exporter .

# Multi-arch
docker buildx build --platform linux/arm64,linux/amd64 -t speedtest-exporter .

# Test (runs 19 tests)
docker build --target test -t speedtest-exporter:test .

# Scan
trivy image --severity CRITICAL,HIGH,MEDIUM speedtest-exporter
```

## Pinned Versions

All base images pinned by manifest list digest, all Rust crates pinned by Cargo.lock, Ookla CLI pinned to v1.2.0.
