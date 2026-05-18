use serde::Deserialize;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct SpeedtestResult {
    pub download_bps: f64,
    pub upload_bps: f64,
    pub ping_latency_seconds: f64,
    pub jitter_seconds: f64,
    pub packet_loss_ratio: f64,
    pub duration_seconds: f64,
    pub server_name: String,
    pub server_country: String,
    pub server_isp: String,
}

#[derive(Debug, Clone)]
pub enum SpeedtestError {
    ExecutionFailed(String),
    ParseFailed(String),
    EmptyOutput,
}

impl std::fmt::Display for SpeedtestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExecutionFailed(msg) => write!(f, "speedtest execution failed: {msg}"),
            Self::ParseFailed(msg) => write!(f, "failed to parse speedtest output: {msg}"),
            Self::EmptyOutput => write!(f, "speedtest produced empty output"),
        }
    }
}

impl std::error::Error for SpeedtestError {}

pub trait SpeedtestRunner: Send + Sync {
    fn run(&self) -> Result<SpeedtestResult, SpeedtestError>;
}

#[derive(Deserialize)]
struct OoklaResult {
    download: DownloadUpload,
    upload: DownloadUpload,
    ping: PingInfo,
    #[serde(rename = "packetLoss")]
    packet_loss: Option<f64>,
    server: ServerInfo,
    #[serde(default)]
    isp: Option<String>,
}

#[derive(Deserialize)]
struct DownloadUpload {
    bandwidth: f64,
}

#[derive(Deserialize)]
struct PingInfo {
    latency: f64,
    jitter: f64,
}

#[derive(Deserialize)]
struct ServerInfo {
    name: String,
    country: String,
}

pub struct OoklaCliRunner {
    server_id: Option<String>,
}

impl OoklaCliRunner {
    pub fn new(server_id: Option<String>) -> Self {
        Self { server_id }
    }
}

impl SpeedtestRunner for OoklaCliRunner {
    fn run(&self) -> Result<SpeedtestResult, SpeedtestError> {
        let start = std::time::Instant::now();

        let mut cmd = std::process::Command::new("speedtest");
        cmd.arg("--accept-license").arg("--format=json");

        if let Some(id) = &self.server_id {
            cmd.arg("--server-id").arg(id);
        }

        let output = cmd
            .output()
            .map_err(|e| SpeedtestError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SpeedtestError::ExecutionFailed(stderr.into_owned()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let last_line = stdout
            .lines()
            .last()
            .ok_or(SpeedtestError::EmptyOutput)?;

        let result: OoklaResult =
            serde_json::from_str(last_line).map_err(|e| SpeedtestError::ParseFailed(e.to_string()))?;

        let elapsed = start.elapsed().as_secs_f64();

        Ok(SpeedtestResult {
            download_bps: result.download.bandwidth,
            upload_bps: result.upload.bandwidth,
            ping_latency_seconds: result.ping.latency / 1000.0,
            jitter_seconds: result.ping.jitter / 1000.0,
            packet_loss_ratio: result.packet_loss.unwrap_or(0.0) / 100.0,
            duration_seconds: elapsed,
            server_name: result.server.name,
            server_country: result.server.country,
            server_isp: result.isp.unwrap_or_default(),
        })
    }
}

// --- Test helpers ---

pub struct MockRunner {
    pub sequence: Mutex<Vec<Result<SpeedtestResult, SpeedtestError>>>,
    pub call_count: Mutex<usize>,
}

impl MockRunner {
    pub fn new(sequence: Vec<Result<SpeedtestResult, SpeedtestError>>) -> Self {
        Self {
            sequence: Mutex::new(sequence),
            call_count: Mutex::new(0),
        }
    }

    pub fn success(result: SpeedtestResult) -> Self {
        Self::new(vec![Ok(result)])
    }

    pub fn error(err: SpeedtestError) -> Self {
        Self::new(vec![Err(err)])
    }

    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

impl SpeedtestRunner for MockRunner {
    fn run(&self) -> Result<SpeedtestResult, SpeedtestError> {
        *self.call_count.lock().unwrap() += 1;
        let seq = self.sequence.lock().unwrap();
        let idx = *self.call_count.lock().unwrap() - 1;
        seq.get(idx)
            .cloned()
            .unwrap_or(Ok(SpeedtestResult {
                download_bps: 0.0,
                upload_bps: 0.0,
                ping_latency_seconds: 0.0,
                jitter_seconds: 0.0,
                packet_loss_ratio: 0.0,
                duration_seconds: 0.0,
                server_name: "mock".into(),
                server_country: "Mockland".into(),
                server_isp: "MockISP".into(),
            }))
    }
}

fn sample_ookla_json() -> &'static str {
    r#"{
        "type": "result",
        "timestamp": "2026-05-18T12:00:00Z",
        "ping": {
            "jitter": 1.221,
            "latency": 6.582,
            "low": 6.354,
            "high": 8.334
        },
        "download": {
            "bandwidth": 90918594,
            "bytes": 891743027,
            "elapsed": 9812,
            "latency": {
                "iqm": 10.17,
                "low": 5.903,
                "high": 99.865,
                "jitter": 13.851
            }
        },
        "upload": {
            "bandwidth": 152648751,
            "bytes": 1234442559,
            "elapsed": 8185,
            "latency": {
                "iqm": 22.082,
                "low": 7.274,
                "high": 121.288,
                "jitter": 19.129
            }
        },
        "packetLoss": 0,
        "isp": "TestISP",
        "server": {
            "id": 8782,
            "name": "TestServer",
            "location": "TestCity",
            "country": "TestCountry",
            "ip": "1.2.3.4"
        }
    }"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ookla_json() {
        let result: OoklaResult = serde_json::from_str(sample_ookla_json()).unwrap();

        assert_eq!(result.download.bandwidth, 90918594.0);
        assert_eq!(result.upload.bandwidth, 152648751.0);
        assert_eq!(result.ping.latency, 6.582);
        assert_eq!(result.ping.jitter, 1.221);
        assert_eq!(result.packet_loss, Some(0.0));
        assert_eq!(result.server.name, "TestServer");
        assert_eq!(result.server.country, "TestCountry");
        assert_eq!(result.isp, Some("TestISP".to_string()));
    }

    #[test]
    fn parses_jsonl_last_line() {
        // Simulate JSONL output: progress line + result line
        let compact_result = r#"{"download":{"bandwidth":90918594},"upload":{"bandwidth":152648751},"ping":{"latency":6.582,"jitter":1.221},"packetLoss":0,"server":{"name":"TestServer","country":"TestCountry"},"isp":"TestISP"}"#;
        let jsonl = format!("{{\"type\":\"progress\"}}\n{}", compact_result);
        let last_line = jsonl.lines().last().unwrap();
        let result: OoklaResult = serde_json::from_str(last_line).unwrap();
        assert_eq!(result.server.name, "TestServer");
    }

    #[test]
    fn mock_runner_returns_configured_result() {
        let result = SpeedtestResult {
            download_bps: 100.0,
            upload_bps: 50.0,
            ping_latency_seconds: 0.01,
            jitter_seconds: 0.001,
            packet_loss_ratio: 0.0,
            duration_seconds: 10.0,
            server_name: "Mock".into(),
            server_country: "Mock".into(),
            server_isp: "Mock".into(),
        };
        let runner = MockRunner::success(result);
        let got = runner.run().unwrap();
        assert_eq!(got.download_bps, 100.0);
        assert_eq!(runner.call_count(), 1);
    }

    #[test]
    fn mock_runner_returns_error() {
        let runner = MockRunner::error(SpeedtestError::EmptyOutput);
        let err = runner.run().unwrap_err();
        assert!(matches!(err, SpeedtestError::EmptyOutput));
    }

    #[test]
    fn speedtest_error_display() {
        let e = SpeedtestError::ExecutionFailed("timeout".into());
        assert!(format!("{e}").contains("timeout"));

        let e = SpeedtestError::ParseFailed("bad json".into());
        assert!(format!("{e}").contains("bad json"));

        let e = SpeedtestError::EmptyOutput;
        assert!(format!("{e}").contains("empty"));
    }
}
