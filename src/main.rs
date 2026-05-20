mod config;
mod logging;
mod metrics;
mod runner;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use prometheus::{Encoder, Registry, TextEncoder};
use tokio::time::interval;
use tracing::{error, info};

use crate::config::Config;
use crate::metrics::{MetricsUpdater, PrometheusMetrics};
use crate::runner::{SpeedtestResult, SpeedtestRunner};

fn handle_result(
    result: Result<SpeedtestResult, crate::runner::SpeedtestError>,
    metrics: &dyn MetricsUpdater,
) {
    match result {
        Ok(r) => {
            metrics.record_success(&r);
            info!(
                download_bps = r.download_bps,
                upload_bps = r.upload_bps,
                ping_seconds = r.ping_latency_seconds,
                server = r.server_name,
                isp = r.server_isp,
                duration = r.duration_seconds,
                "Speedtest completed"
            );
        }
        Err(e) => {
            error!(error = %e, "Speedtest failed");
            metrics.record_failure();
        }
    }
}

async fn run_loop(
    runner: Arc<dyn SpeedtestRunner>,
    metrics: Arc<dyn MetricsUpdater>,
    interval_secs: u64,
) {
    let mut ticker = interval(Duration::from_secs(interval_secs));
    loop {
        ticker.tick().await;
        handle_result(runner.run(), metrics.as_ref());
    }
}

async fn metrics_handler(registry: Registry) -> String {
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap_or_default()
}

#[tokio::main]
async fn main() {
    logging::init();

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Configuration error: {e}");
        std::process::exit(1);
    });

    info!(
        port = config.port,
        interval_secs = config.interval_secs,
        server_id = ?config.server_id,
        "Starting speedtest-exporter"
    );

    let registry = Registry::new();
    let metrics: Arc<dyn MetricsUpdater> = Arc::new(PrometheusMetrics::new(&registry));
    let runner: Arc<dyn SpeedtestRunner> =
        Arc::new(crate::runner::OoklaCliRunner::new(config.server_id));

    tokio::spawn(run_loop(runner, metrics, config.interval_secs));

    let app = Router::new()
        .route(
            "/metrics",
            axum::routing::get(move || metrics_handler(registry.clone())),
        )
        .route("/health", axum::routing::get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();

    info!("Listening on 0.0.0.0:{}", config.port);
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::RecordingMetrics;
    use crate::runner::{MockRunner, SpeedtestError, SpeedtestResult};

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
    fn handle_result_records_success() {
        let metrics = RecordingMetrics::new();
        let result = sample_result();
        handle_result(Ok(result), &metrics);

        let successes = metrics.successes.lock().unwrap();
        assert_eq!(successes.len(), 1);
        assert_eq!(successes[0].download_bps, 100.0);
    }

    #[test]
    fn handle_result_records_failure() {
        let metrics = RecordingMetrics::new();
        handle_result(Err(SpeedtestError::EmptyOutput), &metrics);

        assert_eq!(*metrics.failures.lock().unwrap(), 1);
    }

    #[test]
    fn mock_runner_sequence() {
        let runner = MockRunner::new(vec![
            Ok(sample_result()),
            Err(SpeedtestError::ExecutionFailed("timeout".into())),
            Ok(sample_result()),
        ]);

        let r1 = runner.run().unwrap();
        assert_eq!(r1.download_bps, 100.0);

        let e2 = runner.run().unwrap_err();
        assert!(matches!(e2, SpeedtestError::ExecutionFailed(_)));

        let r3 = runner.run().unwrap();
        assert_eq!(r3.download_bps, 100.0);

        assert_eq!(runner.call_count(), 3);
    }

    #[test]
    fn handle_result_loop_behavior() {
        let runner = MockRunner::new(vec![
            Ok(sample_result()),
            Err(SpeedtestError::ExecutionFailed("timeout".into())),
        ]);
        let metrics = RecordingMetrics::new();

        // Simulate what run_loop does: run -> handle, run -> handle
        handle_result(runner.run(), &metrics);
        handle_result(runner.run(), &metrics);

        assert_eq!(metrics.successes.lock().unwrap().len(), 1);
        assert_eq!(*metrics.failures.lock().unwrap(), 1);
    }
}
