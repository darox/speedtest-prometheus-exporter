# speedtest-exporter

Prometheus exporter for Ookla Speedtest CLI. Runs speedtests on a schedule and serves results at `/metrics`.

## Quickstart

```bash
helm repo add speedtest-exporter https://darox.github.io/speedtest-exporter
helm upgrade --install speedtest-exporter speedtest-exporter/speedtest-exporter
```

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| replicaCount | int | `1` | Number of exporter pods |
| image.repository | string | `ghcr.io/darox/speedtest-exporter` | Container image repository |
| image.pullPolicy | string | `IfNotPresent` | Kubernetes image pull policy |
| image.tag | string | `""` | Image tag (defaults to `.Chart.AppVersion`) |
| image.digest | string | `""` | Image SHA digest (overrides tag) |
| imagePullSecrets | list | `[]` | Secrets for pulling from private registries |
| nameOverride | string | `""` | Override chart name |
| fullnameOverride | string | `""` | Override full release name |
| serviceAccount.create | bool | `true` | Create a ServiceAccount |
| serviceAccount.name | string | `""` | ServiceAccount name (defaults to fullname) |
| serviceAccount.automountServiceAccountToken | bool | `false` | Mount service account token |
| serviceAccount.annotations | object | `{}` | ServiceAccount annotations |
| env.PORT | string | `"9798"` | HTTP listener port |
| env.SPEEDTEST_INTERVAL_SECS | string | `"300"` | Seconds between speedtests (min: 30) |
| env.SPEEDTEST_SERVER_ID | string | `""` | Pin to specific Ookla server |
| env.RUST_LOG | string | `"info"` | Log level |
| extraEnv | list | `[]` | Extra environment variables |
| envFrom | list | `[]` | envFrom references (e.g. secretRef) |
| resources.limits.cpu | string | `"500m"` | CPU limit |
| resources.limits.memory | string | `"256Mi"` | Memory limit |
| resources.requests.cpu | string | `"50m"` | CPU request |
| resources.requests.memory | string | `"128Mi"` | Memory request |
| service.type | string | `"ClusterIP"` | Service type |
| service.port | int | `9798` | Service port |
| service.annotations | object | `{}` | Service annotations |
| serviceMonitor.enabled | bool | `false` | Create a ServiceMonitor |
| serviceMonitor.interval | string | `"30s"` | Scrape interval |
| serviceMonitor.scrapeTimeout | string | `"10s"` | Scrape timeout |
| serviceMonitor.namespace | string | `""` | ServiceMonitor namespace override |
| serviceMonitor.labels | object | `{}` | Additional ServiceMonitor labels |
| serviceMonitor.annotations | object | `{}` | ServiceMonitor annotations |
| networkPolicy.enabled | bool | `false` | Create a NetworkPolicy |
| podSecurityContext | object | see values | Pod-level security context |
| securityContext | object | see values | Container-level security context |
| livenessProbe | object | see values | Liveness probe configuration |
| readinessProbe | object | see values | Readiness probe configuration |
| startupProbe | object | see values | Startup probe configuration |
| podAnnotations | object | `{}` | Pod template annotations |
| nodeSelector | object | `{}` | Node selector constraints |
| tolerations | list | `[]` | Tolerations for tainted nodes |
| affinity | object | `{}` | Scheduling affinity/anti-affinity |

## Security

The chart deploys with a hardened security profile by default:

- Non-root container (`runAsUser: 65534`, `runAsNonRoot: true`)
- Read-only root filesystem
- All capabilities dropped, privilege escalation disabled
- `seccompProfile: RuntimeDefault`
- ServiceAccount with `automountServiceAccountToken: false`
- Optional NetworkPolicy (ingress on metrics port, egress DNS + HTTPS only)
# Helm Chart
