use std::collections::HashMap;
use std::sync::Mutex;

use prometheus::{Gauge, GaugeVec, Opts, Registry};

use crate::runner::SpeedtestResult;

pub trait MetricsUpdater: Send + Sync {
    fn record_success(&self, result: &SpeedtestResult);
    fn record_failure(&self);
}

pub struct PrometheusMetrics {
    download: Gauge,
    upload: Gauge,
    ping_latency: Gauge,
    jitter: Gauge,
    packet_loss: Gauge,
    duration: Gauge,
    success: Gauge,
    server_info: GaugeVec,
    current_server_labels: Mutex<Option<Vec<String>>>,
}

impl PrometheusMetrics {
    pub fn new(registry: &Registry) -> Self {
        let download = Gauge::new(
            "speedtest_download_bps",
            "Download speed in bits per second (converted from Ookla bytes/sec)",
        )
        .unwrap();

        let upload = Gauge::new(
            "speedtest_upload_bps",
            "Upload speed in bits per second (converted from Ookla bytes/sec)",
        )
        .unwrap();

        let ping_latency = Gauge::new(
            "speedtest_ping_latency_seconds",
            "Average ping latency in seconds",
        )
        .unwrap();

        let jitter = Gauge::new("speedtest_jitter_seconds", "Ping jitter in seconds").unwrap();

        let packet_loss = Gauge::new(
            "speedtest_packet_loss_ratio",
            "Packet loss as a ratio (0.0 to 1.0)",
        )
        .unwrap();

        let duration = Gauge::new(
            "speedtest_duration_seconds",
            "Duration of the last speedtest in seconds",
        )
        .unwrap();

        let success = Gauge::new(
            "speedtest_last_success",
            "Whether the last speedtest succeeded (1) or failed (0)",
        )
        .unwrap();

        let server_info = GaugeVec::new(
            Opts::new(
                "speedtest_server_info",
                "Information about the server used for the test",
            ),
            &["server", "country", "isp"],
        )
        .unwrap();

        registry.register(Box::new(download.clone())).unwrap();
        registry.register(Box::new(upload.clone())).unwrap();
        registry.register(Box::new(ping_latency.clone())).unwrap();
        registry.register(Box::new(jitter.clone())).unwrap();
        registry.register(Box::new(packet_loss.clone())).unwrap();
        registry.register(Box::new(duration.clone())).unwrap();
        registry.register(Box::new(success.clone())).unwrap();
        registry.register(Box::new(server_info.clone())).unwrap();

        Self {
            download,
            upload,
            ping_latency,
            jitter,
            packet_loss,
            duration,
            success,
            server_info,
            current_server_labels: Mutex::new(None),
        }
    }
}

impl MetricsUpdater for PrometheusMetrics {
    fn record_success(&self, result: &SpeedtestResult) {
        self.download.set(result.download_bps);
        self.upload.set(result.upload_bps);
        self.ping_latency.set(result.ping_latency_seconds);
        self.jitter.set(result.jitter_seconds);
        self.packet_loss.set(result.packet_loss_ratio);
        self.duration.set(result.duration_seconds);

        if let Some(old_labels) = self.current_server_labels.lock().unwrap().take() {
            let mut labels_map = HashMap::new();
            labels_map.insert("server", old_labels[0].as_str());
            labels_map.insert("country", old_labels[1].as_str());
            labels_map.insert("isp", old_labels[2].as_str());
            let _ = self.server_info.remove(&labels_map);
        }

        let new_labels = vec![
            result.server_name.clone(),
            result.server_country.clone(),
            result.server_isp.clone(),
        ];
        *self.current_server_labels.lock().unwrap() = Some(new_labels);

        self.server_info
            .with_label_values(&[
                &result.server_name,
                &result.server_country,
                &result.server_isp,
            ])
            .set(1.0);

        self.success.set(1.0);
    }

    fn record_failure(&self) {
        self.success.set(0.0);
    }
}

// --- Test adapter ---

#[allow(dead_code)]
pub struct RecordingMetrics {
    pub successes: Mutex<Vec<SpeedtestResult>>,
    pub failures: Mutex<usize>,
}

#[allow(dead_code)]
impl RecordingMetrics {
    pub fn new() -> Self {
        Self {
            successes: Mutex::new(Vec::new()),
            failures: Mutex::new(0),
        }
    }
}

impl MetricsUpdater for RecordingMetrics {
    fn record_success(&self, result: &SpeedtestResult) {
        self.successes.lock().unwrap().push(result.clone());
    }

    fn record_failure(&self) {
        *self.failures.lock().unwrap() += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result() -> SpeedtestResult {
        SpeedtestResult {
            download_bps: 100.0,
            upload_bps: 50.0,
            ping_latency_seconds: 0.01,
            jitter_seconds: 0.001,
            packet_loss_ratio: 0.0,
            duration_seconds: 10.0,
            server_name: "TestServer".into(),
            server_country: "TestCountry".into(),
            server_isp: "TestISP".into(),
        }
    }

    #[test]
    fn recording_metrics_stores_successes() {
        let metrics = RecordingMetrics::new();
        let result = sample_result();
        metrics.record_success(&result);

        let successes = metrics.successes.lock().unwrap();
        assert_eq!(successes.len(), 1);
        assert_eq!(successes[0].download_bps, 100.0);
    }

    #[test]
    fn recording_metrics_counts_failures() {
        let metrics = RecordingMetrics::new();
        metrics.record_failure();
        metrics.record_failure();

        assert_eq!(*metrics.failures.lock().unwrap(), 2);
    }

    #[test]
    fn recording_metrics_multiple_successes() {
        let metrics = RecordingMetrics::new();
        metrics.record_success(&sample_result());
        metrics.record_success(&sample_result());

        assert_eq!(metrics.successes.lock().unwrap().len(), 2);
    }
}
