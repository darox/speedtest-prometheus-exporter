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

## Makefile

All actions go through `make`. No GitHub Actions — everything runs locally.

### Development

| Target | Description |
|---|---|
| `make docker` | Build single-arch image |
| `make docker-multiarch` | Build multi-arch image (amd64 + arm64) |
| `make test` | Run tests (19 tests, builds test target) |
| `make lint` | Run clippy in builder image |
| `make fmt` | Check formatting in builder image |
| `make scan` | Trivy vulnerability scan (CRITICAL, HIGH, MEDIUM) |
| `make audit` | Scan Rust crates for known CVEs (cargo-audit) |
| `make helm-lint` | Lint chart and render templates |

### Local Kind Cluster (pre-release validation)

Deploy to a local kind cluster to verify the image and chart before releasing.

| Target | Description |
|---|---|
| `make kind-create` | Create kind cluster (idempotent, skips if exists) |
| `make kind-load` | Build image, load into kind |
| `make kind-deploy` | Deploy chart with local image via helm |
| `make kind-destroy` | Uninstall chart, delete namespace (keeps cluster) |
| `make kind-clean` | Full teardown: uninstall chart + delete cluster |

Cluster name is configurable: `make kind-create KIND_CLUSTER=my-cluster`.

### Release

```bash
VERSION=v0.0.X make release
```

Ordered pipeline — all local operations complete before any external push. Tag variables (`CLEAN`, `MINOR`, `MAJOR`, `SHA`) computed once by the orchestrator and exported to sub-targets.

| Phase | Target | Description |
|---|---|---|
| Validate | `release-check` | Clean tree, branch check, lint, fmt, test, audit, helm-lint |
| Build | `release-build` | Multi-arch image (local only, tagged `speedtest-exporter:release`) |
| Scan | `release-scan` | Trivy scan against the release image |
| Update | `release-update-chart` | Bump Chart.yaml, README.md, chart/README.md + verify via grep |
| Tag | `release-tag` | Commit chart changes, create git tag (local only, duplicate tag guard) |
| Publish | `release-publish-chart` | Package chart, publish to gh-pages via git worktree (trap cleanup) |
| Push | `release-push` | Tag and push image to GHCR (`vX.Y.Z`, `X.Y`, `X`, `sha-SHORT`), push git tag |

Run `make release-check` for a dry-run validation without any side effects.

**Prerequisites**: `docker` with buildx, `gh` CLI authenticated with `write:packages`, `helm`, clean git working tree, on `main` branch.

## Pinned Versions

All base images pinned by manifest list digest, all Rust crates pinned by Cargo.lock, Ookla CLI pinned to v1.2.0.

## Rules

After every code change, run via makefile:
1. **Lint** — `make lint` must pass
2. **Format** — `make fmt` must pass
3. **Build** — `make docker` must succeed
4. **Test** — `make test` must pass all tests
5. **Scan** — `make scan` must be clean
6. **Audit** — `make audit` must be clean
