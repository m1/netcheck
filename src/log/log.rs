use tracing_subscriber::{EnvFilter, Layer};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub struct Builder {
    level: LevelFilter,
    span_events: bool,
    file: bool,
    flatten_event: bool,
    target: bool,
    span_list: bool,
    thread_names: bool,
    thread_ids: bool,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_level(&mut self, level: LevelFilter) -> &mut Self {
        self.level = level;
        self
    }

    pub fn with_span_events(&mut self, span_events: bool) -> &mut Self {
        self.span_events = span_events;
        self
    }

    pub fn with_file(&mut self, file: bool) -> &mut Self {
        self.file = file;
        self
    }

    pub fn with_flatten_event(&mut self, flatten_event: bool) -> &mut Self {
        self.flatten_event = flatten_event;
        self
    }

    pub fn with_target(&mut self, target: bool) -> &mut Self {
        self.target = target;
        self
    }

    pub fn with_span_list(&mut self, span_list: bool) -> &mut Self {
        self.span_list = span_list;
        self
    }

    pub fn with_thread_names(&mut self, thread_names: bool) -> &mut Self {
        self.thread_names = thread_names;
        self
    }

    pub fn with_thread_ids(&mut self, thread_ids: bool) -> &mut Self {
        self.thread_ids = thread_ids;
        self
    }

    pub fn build(&mut self) {
        let env = EnvFilter::builder()
            .with_default_directive(self.level.into()).parse("")
            .expect("Failed to parse filter");

        let layer = tracing_subscriber::fmt::layer()
            .json()
            .with_timer(UtcTime::rfc_3339())
            .flatten_event(self.flatten_event)
            .with_target(self.target)
            .with_level(true)
            .with_thread_ids(self.thread_ids)
            .with_current_span(true)
            .with_span_list(self.span_list)
            .with_level(true)
            .with_file(self.file)
            .with_line_number(true)
            .with_ansi(false)
            .with_span_events(if self.span_events { FmtSpan::ENTER | FmtSpan::EXIT | FmtSpan::CLOSE } else { FmtSpan::NONE })
            .with_thread_names(self.thread_names)
            .log_internal_errors(true)
            .with_filter(env);

        tracing_subscriber::registry().with(layer).init();
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder {
            level: LevelFilter::INFO,
            span_events: true,
            thread_ids: true,
            thread_names: true,
            span_list: true,
            target: true,
            flatten_event: true,
            file: true,
        }
    }
}

pub struct Logger {}

impl Logger {
    pub fn builder() -> Builder {
        Builder::default()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_builder() {
        let mut binding = Builder::new();
        let builder = binding.with_level(LevelFilter::INFO)
            .with_span_events(false)
            .with_file(false)
            .with_flatten_event(false)
            .with_target(false)
            .with_span_list(false)
            .with_thread_names(false)
            .with_thread_ids(false);

        builder.build();

        assert_eq!(builder.level, LevelFilter::INFO);
        assert_eq!(builder.span_events, false);
        assert_eq!(builder.file, false);
        assert_eq!(builder.flatten_event, false);
        assert_eq!(builder.target, false);
        assert_eq!(builder.span_list, false);
        assert_eq!(builder.thread_names, false);
        assert_eq!(builder.thread_ids, false);
    }
}
