//! Standard telemetry dashboard payload and live metric synchronization for template applications.

pub use spodeian_telemetry::{TelemetryChannelMetric, TelemetryDashboardData};

/// Builds a default telemetry dashboard populated with current `spodeian-telemetry` snapshots.
pub fn default_template_telemetry() -> TelemetryDashboardData {
    let mut data = TelemetryDashboardData {
        title: "Client Application Telemetry".to_string(),
        active_backend: "Local Storage / IndexedDB".to_string(),
        ..Default::default()
    };
    refresh_telemetry_dashboard(&mut data, "Local Storage / IndexedDB", 0);
    data
}

/// Synchronizes `data` with live `spodeian_telemetry::metrics_snapshot()` counters, active
/// storage backend label, and managed item count.
pub fn refresh_telemetry_dashboard(
    data: &mut TelemetryDashboardData,
    active_backend: &str,
    items_count: usize,
) {
    let metrics = spodeian_telemetry::metrics_snapshot();
    let uptime_secs = metrics.uptime_millis / 1000;
    let total_events = metrics.total_events() as usize;
    let total_ops = total_events.max(items_count);

    let success_rate = if total_events == 0 {
        100.0
    } else {
        let non_error = total_events.saturating_sub(metrics.errors as usize);
        (non_error as f64 / total_events as f64) * 100.0
    };

    let throughput_ops_sec = if uptime_secs == 0 {
        total_ops as f64
    } else {
        total_ops as f64 / uptime_secs as f64
    };

    data.uptime_secs = uptime_secs;
    data.total_operations = total_ops;
    data.success_rate = success_rate;
    data.throughput_ops_sec = throughput_ops_sec;
    data.active_backend = active_backend.to_string();
    data.channels = vec![
        TelemetryChannelMetric {
            name: "Managed Working Set".to_string(),
            count: items_count,
            latency_ms: 0.05,
            status: "Healthy".to_string(),
        },
        TelemetryChannelMetric {
            name: "Telemetry Ring Buffer".to_string(),
            count: metrics.buffered_entries,
            latency_ms: 0.02,
            status: if metrics.errors == 0 {
                "Optimal".to_string()
            } else {
                "Warnings/Errors Logged".to_string()
            },
        },
        TelemetryChannelMetric {
            name: "Tracing Events Total".to_string(),
            count: total_events,
            latency_ms: 0.01,
            status: if metrics.errors == 0 {
                "Healthy".to_string()
            } else {
                "Degraded".to_string()
            },
        },
    ];
}
