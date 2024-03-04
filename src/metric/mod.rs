pub use self::metric::KEY_EVENTS;
pub use self::metric::KEY_REQUESTS;
pub use self::metric::KEY_REQUESTS_RESPONSE_TIME_NS;
pub use self::metric::KEY_TARGET_STATUS;
pub use self::metric::LABEL_EVENTS_STATUS;
pub use self::metric::LABEL_EVENTS_TARGET_NAME;
pub use self::metric::LABEL_EVENTS_TARGET_URL;
pub use self::metric::LABEL_TARGET_URLS;
pub use self::metric::register_metrics;
pub use self::metric::VALUE_EVENT_STATUS_UNAVAILABLE;
pub use self::metric::VALUE_EVENT_STATUS_UNAVAILABLE_TO_AVAILABLE;
pub use self::metric::VALUE_EVENT_STATUS_FAILURE;
pub use self::metric::VALUE_EVENT_STATUS_SUCCESS;
pub use self::metric::VALUE_EVENT_STATUS_AVAILABLE;
pub use self::metric::VALUE_EVENT_STATUS_AVAILABLE_TO_UNAVAILABLE;

mod metric;
