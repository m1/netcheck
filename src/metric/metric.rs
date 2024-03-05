use metrics::{describe_counter, describe_gauge, describe_histogram, gauge, Unit};
use metrics_exporter_prometheus::PrometheusBuilder;

use crate::built_info;

pub const KEY_NETCHECK_RUNNING_STATUS: &str = "netcheck_running_status";
pub const KEY_EVENTS: &str = "netcheck_events";
pub const KEY_REQUESTS: &str = "netcheck_requests";
pub const KEY_REQUESTS_RESPONSE_TIME_NS: &str = "netcheck_requests_response_time_ns";
pub const KEY_TARGET_STATUS: &str = "netcheck_target_status";

pub const LABEL_EVENTS_STATUS: &str = "status";
pub const LABEL_EVENTS_TARGET_NAME: &str = "target_name";
pub const LABEL_EVENTS_TARGET_URL: &str = "url";
pub const LABEL_TARGET_URLS: &str = "urls";
pub const LABEL_NETCHECK_VERSION: &str = "version";
pub const LABEL_NETCHECK_STARTED_AT: &str = "started_at";

pub const VALUE_EVENT_STATUS_SUCCESS: &str = "success";
pub const VALUE_EVENT_STATUS_FAILURE: &str = "failure";
pub const VALUE_EVENT_STATUS_AVAILABLE_TO_UNAVAILABLE: &str = "available_to_unavailable";
pub const VALUE_EVENT_STATUS_UNAVAILABLE_TO_AVAILABLE: &str = "unavailable_to_available";
pub const VALUE_EVENT_STATUS_AVAILABLE: &str = "available";
pub const VALUE_EVENT_STATUS_UNAVAILABLE: &str = "unavailable";

pub fn register_metrics(port: Option<u16>) {
    let port = port.unwrap_or(9000);
    let builder = PrometheusBuilder::new().with_http_listener(([127, 0, 0, 1], port));
    builder
        .install()
        .expect("failed to install Prometheus recorder");

    describe_gauge!(
        KEY_NETCHECK_RUNNING_STATUS,
        "The status of the netcheck service"
    );
    describe_counter!(
        KEY_REQUESTS,
        Unit::Count,
        "Counter of how many requests have taken place"
    );
    describe_counter!(
        KEY_EVENTS,
        Unit::Count,
        "Counter of how many events have taken place"
    );
    describe_gauge!(KEY_TARGET_STATUS, "The status of the target");
    describe_histogram!(
        KEY_REQUESTS_RESPONSE_TIME_NS,
        Unit::Nanoseconds,
        "The time taken to get a response from a request"
    );

    gauge!(
        KEY_NETCHECK_RUNNING_STATUS,
        LABEL_NETCHECK_VERSION => built_info::PKG_VERSION,
        LABEL_NETCHECK_STARTED_AT => chrono::Utc::now().to_rfc3339(),
    )
    .increment(1.);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_metrics() {
        register_metrics(Some(9001));
        let requests = metrics::counter!(KEY_REQUESTS);
        let events = metrics::counter!(KEY_EVENTS);
        let target_status = metrics::gauge!(KEY_TARGET_STATUS);
        let response_time = metrics::histogram!(KEY_REQUESTS_RESPONSE_TIME_NS);

        requests.increment(1);
        target_status.increment(1.);
        events.increment(1);
        response_time.record(1.);
    }
}
