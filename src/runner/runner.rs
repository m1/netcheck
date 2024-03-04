use std::time::{Duration, Instant};

use metrics::{counter, gauge, histogram};
use reqwest::{Client, Url};
use thiserror::Error;
use tokio::{task, time};
use tracing::{debug, info};

use crate::built_info;
use crate::metric::{
    KEY_EVENTS,
    KEY_REQUESTS,
    KEY_REQUESTS_RESPONSE_TIME_NS,
    KEY_TARGET_STATUS,
    LABEL_EVENTS_STATUS,
    LABEL_EVENTS_TARGET_NAME,
    LABEL_EVENTS_TARGET_URL,
    LABEL_TARGET_URLS,
    VALUE_EVENT_STATUS_AVAILABLE,
    VALUE_EVENT_STATUS_AVAILABLE_TO_UNAVAILABLE,
    VALUE_EVENT_STATUS_FAILURE,
    VALUE_EVENT_STATUS_SUCCESS,
    VALUE_EVENT_STATUS_UNAVAILABLE,
    VALUE_EVENT_STATUS_UNAVAILABLE_TO_AVAILABLE,
};
use crate::runner::{Event, Status};
use crate::runner::target::Target;
use crate::runner::url::vec_to_string;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error("{source}")]
    TokioError {
        #[from]
        source: task::JoinError,
    },
    #[error("{status}")]
    StatusError { status: u16 },
}

/// Runner is a struct that runs a check on a target.
#[derive(Clone, Debug)]
pub struct Runner {
    target: Target,
    connect_timeout_ms: u64,
    timeout_ms: u64,
    wait_time_seconds: u64,
    pub failure_threshold: u8,
    run_for_seconds: Option<u64>,
    run_for_iterations: Option<u64>,
    user_agent: String,
}

impl Runner {
    /// Run the check on the target.
    #[tracing::instrument(level = "info")]
    pub async fn run<>(&self) -> Result<(), Error> {
        let client = self.get_client()?;
        let runner = self.clone();
        let urls = self.target.urls.clone();
        let wait = self.wait_time_seconds.clone();
        let mut status = Status::new(self.failure_threshold);

        let forever = task::spawn(async move {
            runner.tick(urls, client, wait, &mut status).await;
        });

        forever.await?;

        Ok(())
    }

    /// Run the check on the target.
    ///
    /// # Arguments
    ///
    /// * `urls`: The urls to check.
    /// * `client`: The client to use for the check.
    /// * `wait`: The time to wait between checks.
    /// * `status`: The global status lock.
    ///
    /// returns: ()
    #[tracing::instrument(level = "debug")]
    async fn tick(&self, urls: Vec<Url>, client: Client, wait: u64, status: &mut Status) {
        let mut interval = time::interval(Duration::from_secs(wait));
        let mut idx = 0;
        let target_name = self.target.name.clone();
        let started = Instant::now();
        let mut iterations = 0;
        let mut available_count = 0;

        loop {
            if self.should_stop(started, iterations) {
                break;
            }

            if idx >= urls.len() {
                info!(runner_target = target_name, "target tick complete, {}/{} available", available_count, urls.len());
                idx = 0;
                available_count = 0;
            }

            let url = &urls[idx];
            let is_available = self.check_url(url.clone(), client.clone(), status, target_name.clone()).await;

            available_count += if is_available { 1 } else { 0 };
            iterations += 1;
            idx += 1;
            interval.tick().await;
        }
    }

    /// Get the client to use for the check.
    fn get_client(&self) -> reqwest::Result<Client> {
        Client::builder()
            .user_agent(self.user_agent.as_str())
            .timeout(Duration::from_millis(self.timeout_ms))
            .connect_timeout(Duration::from_millis(self.connect_timeout_ms))
            .build()
    }

    #[tracing::instrument(level = "debug")]
    async fn check_url(&self, url: Url, client: Client, status: &mut Status, target: String) -> bool {
        let start = Instant::now();
        let resp = client.get(url.as_str())
            .send()
            .await;

        return match resp {
            Ok(resp) => {
                if resp.status().is_server_error() || resp.status().is_client_error() {
                    let status_error = Error::StatusError { status: resp.status().as_u16() };
                    self.handle_response_error(status_error, target, url, start, status).await;

                    return false;
                }

                self.handle_response_ok(target, url, start, status).await;

                true
            }
            Err(err) => {
                self.handle_response_error(Error::from(err), target, url, start, status).await;

                false
            }
        };
    }

    fn should_stop(&self, started: Instant, iterations: i32) -> bool {
        if let Some(run_for_iterations) = self.run_for_iterations {
            if iterations >= run_for_iterations as i32 {
                return true;
            }
        }

        if let Some(run_for_seconds) = self.run_for_seconds {
            if started.elapsed().as_secs() >= run_for_seconds {
                return true;
            }
        }

        false
    }

    async fn handle_response_error(&self, err: Error, target: String, url: Url, start: Instant, status: &mut Status) {
        debug!(runner_target = target, url = url.to_string(), err = err.to_string(), resp_ns = start.elapsed().as_nanos(), "tick failure");
        self.update_request_metrics(false, &start, target.clone(), url.clone());

        match status.handle_unavailable() {
            Event::AvailableToUnavailable => {
                counter!(KEY_EVENTS,
                            LABEL_EVENTS_STATUS => VALUE_EVENT_STATUS_AVAILABLE_TO_UNAVAILABLE,
                    LABEL_EVENTS_TARGET_NAME => target.clone(),
                ).increment(1);
                info!(runner_target = target, url = url.to_string(), "available to unavailable");
            }
            _ => {
                if status.is_unavailable {
                    let diff = chrono::Utc::now().signed_duration_since(status.unavailable_started);
                    info!(runner_target = target, url = url.to_string(), time_delta = diff.to_string(), "still unavailable, unavailable for {}s", diff.num_seconds());
                }
            }
        }
    }

    async fn handle_response_ok(&self, target: String, url: Url, start: Instant, status: &mut Status) {
        debug!(runner_target = target, url = url.to_string(), resp_ns = start.elapsed().as_nanos(), "tick success");
        self.update_request_metrics(true, &start, target.clone(), url.clone());

        match status.handle_available() {
            Event::UnavailableToAvailable(diff) => {
                counter!(KEY_EVENTS,
                            LABEL_EVENTS_STATUS => VALUE_EVENT_STATUS_UNAVAILABLE_TO_AVAILABLE,
                            LABEL_EVENTS_TARGET_NAME => target.clone(),
                        ).increment(1);
                info!(runner_target = target, url = url.to_string(), diff = diff.num_seconds(), "unavailable to available");
            }
            _ => {}
        }
    }

    /// Update the request metrics.
    ///
    /// # Arguments
    ///
    /// * `success`: If the request was successful.
    /// * `started`: When the request started.
    /// * `target`: The target name.
    /// * `url`:  The target url.
    fn update_request_metrics(&self, success: bool, started: &Instant, target: String, url: Url) {
        let status = if success {
            VALUE_EVENT_STATUS_SUCCESS
        } else {
            VALUE_EVENT_STATUS_FAILURE
        };

        counter!(
            KEY_REQUESTS,
            LABEL_EVENTS_STATUS => status,
            LABEL_EVENTS_TARGET_NAME => target.to_string(),
            LABEL_EVENTS_TARGET_URL => url.to_string(),
        ).increment(1);

        histogram!(
            KEY_REQUESTS_RESPONSE_TIME_NS,
            LABEL_EVENTS_STATUS => status,
            LABEL_EVENTS_TARGET_NAME => target.to_string(),
            LABEL_EVENTS_TARGET_URL => url.to_string(),
        ).record(started.elapsed().as_nanos() as f64);

        gauge!(
            KEY_TARGET_STATUS,
            LABEL_EVENTS_TARGET_NAME => target.to_string(),
            LABEL_EVENTS_STATUS => VALUE_EVENT_STATUS_AVAILABLE,
            LABEL_TARGET_URLS => vec_to_string(self.target.urls.clone()),
        ).set(if success { 1. } else { 0. });

        gauge!(
            KEY_TARGET_STATUS,
            LABEL_EVENTS_TARGET_NAME => target.to_string(),
            LABEL_EVENTS_STATUS => VALUE_EVENT_STATUS_UNAVAILABLE,
            LABEL_TARGET_URLS => vec_to_string(self.target.urls.clone()),
        ).set(if success { 0. } else { 1. });
    }
}

/// RunnerBuilder is a struct that builds a Runner.
///
/// # Example
///
/// ```
/// # use netcheck::runner::{RunnerBuilder, Target};
/// # use reqwest::Url;
///
/// let runner = RunnerBuilder::new()
///     .target(Target::new("external".to_string(), vec![Url::parse("https://example.com").unwrap()]))
///     .connect_timeout_ms(1000)
///     .timeout_ms(1000)
///     .wait_time_seconds(1)
///     .build();
/// ```
pub struct RunnerBuilder {
    target: Target,
    connect_timeout_ms: u64,
    timeout_ms: u64,
    wait_time_seconds: u64,
    failure_threshold: u8,
    run_for_seconds: Option<u64>,
    run_for_iterations: Option<u64>,
    user_agent: Option<String>,
}

impl RunnerBuilder {
    /// Create a new RunnerBuilder.
    ///
    /// returns: RunnerBuilder
    ///
    /// # Example
    ///
    /// ```
    /// # use netcheck::runner::RunnerBuilder;
    /// let runner = RunnerBuilder::new();
    /// ```
    pub fn new() -> RunnerBuilder {
        RunnerBuilder::default()
    }

    /// Set the target of the RunnerBuilder.
    pub fn target(mut self, target: Target) -> RunnerBuilder {
        self.target = target;
        self
    }

    /// Set the connect_timeout_ms of the RunnerBuilder.
    pub fn connect_timeout_ms(mut self, connect_timeout_ms: u64) -> RunnerBuilder {
        self.connect_timeout_ms = connect_timeout_ms;
        self
    }

    /// Set the timeout_ms of the RunnerBuilder.
    pub fn timeout_ms(mut self, timeout_ms: u64) -> RunnerBuilder {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set the wait_time_seconds of the RunnerBuilder.
    pub fn wait_time_seconds(mut self, wait_time_seconds: u64) -> RunnerBuilder {
        self.wait_time_seconds = wait_time_seconds;
        self
    }

    /// Set the failure_threshold of the RunnerBuilder.
    pub fn failure_threshold(mut self, failure_threshold: u8) -> RunnerBuilder {
        self.failure_threshold = failure_threshold;
        self
    }

    /// Set the run_for_seconds of the RunnerBuilder.
    pub fn run_for_seconds(mut self, run_for_seconds: u64) -> RunnerBuilder {
        self.run_for_seconds = Some(run_for_seconds);
        self
    }

    /// Set the run_for_iterations of the RunnerBuilder.
    pub fn run_for_iterations(mut self, run_for_iterations: u64) -> RunnerBuilder {
        self.run_for_iterations = Some(run_for_iterations);
        self
    }

    pub fn user_agent(mut self, user_agent: String) -> RunnerBuilder {
        self.user_agent = Some(user_agent);
        self
    }

    /// Build the Runner.
    ///
    /// returns: Runner
    ///
    /// # Example
    ///
    /// ```
    /// # use netcheck::runner::{RunnerBuilder, Target};
    /// # use reqwest::Url;
    /// let runner = RunnerBuilder::new()
    ///     .target(Target::new("external".to_string(), vec![Url::parse("https://example.com").unwrap()]))
    ///     .connect_timeout_ms(1000)
    ///     .timeout_ms(1000)
    ///     .wait_time_seconds(1)
    ///     .build();
    /// ```
    pub fn build(self) -> Runner {
        Runner {
            target: self.target,
            connect_timeout_ms: self.connect_timeout_ms,
            timeout_ms: self.timeout_ms,
            wait_time_seconds: self.wait_time_seconds,
            failure_threshold: self.failure_threshold,
            run_for_seconds: self.run_for_seconds,
            run_for_iterations: self.run_for_iterations,
            user_agent: self.user_agent.unwrap_or_else(get_user_agent),
        }
    }
}

impl Default for RunnerBuilder {
    fn default() -> Self {
        RunnerBuilder {
            target: Target {
                name: "external".to_string(),
                urls: vec![
                    Url::parse("https://1.1.1.1").expect("default url not valid"),
                    Url::parse("https://dns.google").expect("default url not valid"),
                ],
            },
            connect_timeout_ms: 1000,
            timeout_ms: 1000,
            wait_time_seconds: 1,
            failure_threshold: 5,
            run_for_seconds: None,
            run_for_iterations: None,
            user_agent: None,
        }
    }
}

fn get_user_agent() -> String {
    format!("{}/{}", built_info::PKG_NAME, built_info::PKG_VERSION)
}

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;
    use pretty_assertions::assert_eq;
    use reqwest::Url;

    use crate::built_info;
    use crate::runner::{RunnerBuilder, Status, Target};

    #[test]
    fn test_runner_builder() {
        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse("https://example.com").unwrap()]))
            .connect_timeout_ms(1000)
            .timeout_ms(1000)
            .failure_threshold(1)
            .wait_time_seconds(1)
            .run_for_seconds(1)
            .run_for_iterations(1)
            .user_agent("test".to_string())
            .build();

        assert_eq!(runner.target.name, "external");
        assert_eq!(runner.target.urls[0], Url::parse("https://example.com").unwrap());
        assert_eq!(runner.connect_timeout_ms, 1000);
        assert_eq!(runner.timeout_ms, 1000);
        assert_eq!(runner.failure_threshold, 1);
        assert_eq!(runner.wait_time_seconds, 1);
        assert_eq!(runner.run_for_seconds, Some(1));
        assert_eq!(runner.run_for_iterations, Some(1));
        assert_eq!(runner.user_agent, "test");
    }

    #[test]
    fn test_runner_builder_default() {
        let runner = RunnerBuilder::default().build();

        assert_eq!(runner.connect_timeout_ms, 1000);
        assert_eq!(runner.timeout_ms, 1000);
        assert_eq!(runner.wait_time_seconds, 1);
        assert_eq!(runner.failure_threshold, 5);
        assert_eq!(runner.run_for_seconds, None);
        assert_eq!(runner.run_for_iterations, None);
        assert_eq!(runner.user_agent, format!("{}/{}", built_info::PKG_NAME, built_info::PKG_VERSION));
    }

    #[test]
    fn test_runner_builder_target() {
        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse("https://example.com").unwrap()]))
            .build();

        assert_eq!(runner.target.name, "external");
        assert_eq!(runner.target.urls[0], Url::parse("https://example.com").unwrap());
    }

    #[test]
    fn test_runner_builder_connect_timeout_ms() {
        let runner = RunnerBuilder::new()
            .connect_timeout_ms(1000)
            .build();

        assert_eq!(runner.connect_timeout_ms, 1000);
    }

    #[test]
    fn test_runner_builder_timeout_ms() {
        let runner = RunnerBuilder::new()
            .timeout_ms(1000)
            .build();

        assert_eq!(runner.timeout_ms, 1000);
    }

    #[test]
    fn test_runner_builder_wait_time_seconds() {
        let runner = RunnerBuilder::new()
            .wait_time_seconds(1)
            .build();

        assert_eq!(runner.wait_time_seconds, 1);
    }

    #[test]
    fn test_runner_builder_failure_threshold() {
        let runner = RunnerBuilder::new()
            .failure_threshold(1)
            .build();

        assert_eq!(runner.failure_threshold, 1);
    }

    #[test]
    fn test_runner_builder_run_for_seconds() {
        let runner = RunnerBuilder::new()
            .run_for_seconds(1)
            .build();

        assert_eq!(runner.run_for_seconds, Some(1));
    }

    #[test]
    fn test_runner_builder_run_for_iterations() {
        let runner = RunnerBuilder::new()
            .run_for_iterations(1)
            .build();

        assert_eq!(runner.run_for_iterations, Some(1));
    }

    #[test]
    fn test_runner_builder_user_agent() {
        let runner = RunnerBuilder::new()
            .user_agent("test".to_string())
            .build();

        assert_eq!(runner.user_agent, "test");
    }

    #[test]
    fn test_runner_get_user_agent() {
        let user_agent = super::get_user_agent();
        assert_eq!(user_agent, format!("{}/{}", built_info::PKG_NAME, built_info::PKG_VERSION));
    }

    #[tokio::test]
    async fn test_runner_run() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .run_for_iterations(1)
            .build();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200);
        });

        runner.run().await.unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn test_runner_run_timeout() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .run_for_iterations(1)
            .timeout_ms(10)
            .connect_timeout_ms(10)
            .build();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200)
                .delay(std::time::Duration::from_secs(1));
        });

        runner.run().await.unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn test_runner_check_url_available() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .build();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200);
        });

        let status = &mut Status::new(5);
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;

        assert_eq!(status.is_unavailable, false);
        assert_eq!(status.unavailable_counted, 0);
        assert_eq!(status.available_counted, 1);

        mock.assert();
    }

    #[tokio::test]
    async fn test_runner_check_url_unavailable_no_path() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .build();

        let status = &mut Status::new(5);
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;

        assert_eq!(status.is_unavailable, false);
        assert_eq!(status.unavailable_counted, 1);
        assert_eq!(status.available_counted, 0);
    }

    #[tokio::test]
    async fn test_runner_check_url_unavailable_500() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .build();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(500);
        });

        let status = &mut Status::new(5);
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;

        assert_eq!(status.is_unavailable, false);
        assert_eq!(status.unavailable_counted, 1);
        assert_eq!(status.available_counted, 0);

        mock.assert();
    }

    #[tokio::test]
    async fn test_runner_check_url_unavailable_400() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .build();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(400);
        });

        let status = &mut Status::new(5);
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;

        assert_eq!(status.is_unavailable, false);
        assert_eq!(status.unavailable_counted, 1);
        assert_eq!(status.available_counted, 0);

        mock.assert();
    }

    #[tokio::test]
    async fn test_runner_check_url_unavailable_status() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .build();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(500);
        });

        let status = &mut Status::new(2);
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;

        assert_eq!(status.is_unavailable, true);
        assert_eq!(status.unavailable_counted, 3);
        assert_eq!(status.available_counted, 0);

        mock.assert_hits(3);
    }

    #[tokio::test]
    async fn test_runner_check_url_available_status() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .build();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200);
        });

        let status = &mut Status::new(2);
        status.is_unavailable = true;
        status.unavailable_started = chrono::Utc::now();
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;
        runner.check_url(Url::parse(&url).unwrap(), reqwest::Client::new(), status, "test".to_string()).await;

        assert_eq!(status.is_unavailable, false);
        assert_eq!(status.available_counted, 3);
        assert_eq!(status.unavailable_events.len(), 1);

        mock.assert_hits(3);
    }

    #[tokio::test]
    async fn test_runner_check_url_timeout() {
        let server = MockServer::start();
        let url = server.url("/");

        let runner = RunnerBuilder::new()
            .target(Target::new("external".to_string(), vec![Url::parse(&url).unwrap()]))
            .connect_timeout_ms(10)
            .timeout_ms(10)
            .build();

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200)
                .delay(std::time::Duration::from_secs(1));
        });

        let client = runner.get_client().expect("failed to get client");

        let status = &mut Status::new(2);
        runner.check_url(Url::parse(&url).unwrap(), client.clone(), status, "test".to_string()).await;

        assert_eq!(status.unavailable_counted, 1);

        mock.assert();
    }
}
