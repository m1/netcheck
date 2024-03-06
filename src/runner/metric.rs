use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram, ObservableGauge, Unit};

pub const METRIC_LABEL_STATUS: &str = "status";
pub const METRIC_LABEL_TARGET_NAME: &str = "target_name";
pub const METRIC_LABEL_URL: &str = "url";
pub const METRIC_LABEL_URLS: &str = "urls";
pub const METRIC_LABEL_RUNNER_VERSION: &str = "runner_version";
pub const METRIC_LABEL_RUNNER_STARTED_AT: &str = "started_at";

pub const METRIC_VALUE_AVAILABLE_TO_UNAVAILABLE: &str = "available_to_unavailable";
pub const METRIC_VALUE_UNAVAILABLE_TO_AVAILABLE: &str = "unavailable_to_available";
pub const METRIC_VALUE_AVAILABLE: &str = "available";
pub const METRIC_VALUE_UNAVAILABLE: &str = "unavailable";

#[derive(Clone, Debug)]
pub struct Metrics {
    pub status: ObservableGauge<u64>,
    pub events: Counter<u64>,
    pub requests: Counter<u64>,
    pub target_status: ObservableGauge<u64>,
    pub requests_response_time_ns: Histogram<f64>,
}

impl Default for Metrics {
    fn default() -> Self {
        let meter = global::meter("netcheck_runner");

        Metrics {
            status: meter
                .u64_observable_gauge("runner_status")
                .with_description("The status of the netcheck service")
                .with_unit(Unit::new("count"))
                .init(),
            events: meter
                .u64_counter("runner_events")
                .with_description("Counter of how many events have taken place")
                .with_unit(Unit::new("count"))
                .init(),
            requests: meter
                .u64_counter("runner_requests")
                .with_description("Counter of how many requests have taken place")
                .with_unit(Unit::new("count"))
                .init(),
            target_status: meter
                .u64_observable_gauge("runner_target_status")
                .with_description("The status of the target")
                .with_unit(Unit::new("count"))
                .init(),
            requests_response_time_ns: meter
                .f64_histogram("runner_requests_response_time_ns")
                .with_description("The time taken to get a response from a request")
                .with_unit(Unit::new("ns"))
                .init(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::built_info;
    use opentelemetry::KeyValue;

    #[test]
    fn test_metrics_default_with_labels() {
        let metrics = Metrics::default();
        metrics.status.observe(
            1,
            &[
                KeyValue::new(METRIC_LABEL_RUNNER_VERSION, built_info::PKG_VERSION),
                KeyValue::new(
                    METRIC_LABEL_RUNNER_STARTED_AT,
                    chrono::Utc::now().to_rfc3339(),
                ),
            ],
        );
    }
}
