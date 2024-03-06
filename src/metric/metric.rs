use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use actix_web::{App, HttpServer, web};
use actix_web_opentelemetry::{PrometheusMetricsHandler, RequestMetrics};
use opentelemetry::{global, KeyValue};
use opentelemetry::metrics::MetricsError;
use opentelemetry_sdk::{
    metrics::MeterProviderBuilder

    ,
    Resource,
};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_semantic_conventions::{
    resource::{SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use opentelemetry_semantic_conventions::resource::TELEMETRY_SDK_LANGUAGE;
use thiserror::Error;
use tokio::task;
use tracing::info;

use crate::built_info;

const DEFAULT_PORT: u16 = 8080;

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
    #[error("{source}")]
    MetricsError {
        #[from]
        source: MetricsError,
    },
    #[error("{status}")]
    StatusError { status: u16 },
    #[error("{source}")]
    IO {
        #[from]
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub struct MetricProvider {
    pub metrics_handler: PrometheusMetricsHandler,
    pub meter_provider: SdkMeterProvider,
}

impl MetricProvider {
    #[tracing::instrument(level = "debug")]
    pub fn new() -> Self {
        let registry = prometheus::Registry::new();
        let exporter = opentelemetry_prometheus::exporter()
            .with_registry(registry.clone())
            .with_namespace("netcheck")
            .build()
            .expect("failed to build prometheus exporter");
        let meter_provider = MeterProviderBuilder::default()
            .with_reader(exporter)
            .with_resource(Resource::new(vec![
                KeyValue::new(SERVICE_NAME, "netcheck"),
                KeyValue::new(SERVICE_VERSION, built_info::PKG_VERSION),
                KeyValue::new(TELEMETRY_SDK_LANGUAGE, "rust"),
                KeyValue::new(SCHEMA_URL, "https://opentelemetry.io/schemas/1.7.0"),
            ]))
            .build();
        global::set_meter_provider(meter_provider.clone());
        let metrics_handler = PrometheusMetricsHandler::new(registry);
        Self {
            metrics_handler,
            meter_provider,
        }
    }

    #[tracing::instrument(level = "debug")]
    pub async fn listen(&self, port: Option<u16>) -> Result<(), Error> {
        let port = port.unwrap_or(DEFAULT_PORT);
        let meter_provider = self.meter_provider.clone();
        let metrics_handler = self.metrics_handler.clone();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port));

        HttpServer::new(move || {
            let app = App::new()
                .wrap(RequestMetrics::default());
            let app = app.route("/metrics", web::get().to(metrics_handler.clone()));
            app
        })
            .bind(addr)?
            .run()
            .await?;

        info!("metrics server listening on port {}", port);
        meter_provider.shutdown()?;

        Ok(())
    }
}
