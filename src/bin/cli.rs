use std::sync::{Arc, Mutex};

use clap::{Args, Parser, Subcommand};
use thiserror::Error;
use tracing_subscriber::filter::LevelFilter;

use netcheck::runner;
use netcheck::{log, metric};

#[derive(Parser, Debug)]
#[command(name = "netcheck")]
#[command(author = "Miles Croxford <hello@milescroxford.com>")]
#[command(version = "0.0.1")]
#[command(about = "Netcheck checks the network")]
#[command(long_about = "Netcheck checks the network and reports back on the status of the network")]
struct Cli {
    #[arg(short = 'D')]
    #[arg(long)]
    #[arg(global = true)]
    debug: Option<bool>,

    #[arg(short = 'v')]
    #[arg(long)]
    #[arg(global = true)]
    verbose: Option<bool>,

    #[arg(long)]
    #[arg(help = "The level to log at")]
    #[arg(global = true)]
    log_level: Option<LevelFilter>,

    #[arg(long)]
    #[arg(help = "Port to expose metrics on")]
    #[arg(global = true)]
    metrics_port: Option<u16>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, PartialEq, Debug)]
enum Commands {
    Run(Run),
}

#[derive(Args, PartialEq, Debug)]
#[command(about = "Runs the netcheck service")]
#[command(long_about = "Runs the netcheck service and checks the network using the passed targets")]
struct Run {
    #[arg(short)]
    #[arg(long)]
    #[arg(help = "List of targets to check if a network connection is attainable")]
    #[arg(default_value = "external=https://one.one.one.one,https://dns.google")]
    target: Vec<runner::Target>,

    #[arg(long = "connect")]
    #[arg(help = "Connect timeout milliseconds to be considered a failure")]
    #[arg(default_value = "500")]
    connect_timeout_ms: u64,

    #[arg(long = "timeout")]
    #[arg(help = "Timeout milliseconds to be considered a failure")]
    #[arg(default_value = "500")]
    timeout_ms: u64,

    #[arg(short = 'w')]
    #[arg(long = "wait")]
    #[arg(help = "Time to wait between requests in seconds")]
    #[arg(default_value = "2")]
    wait_time_seconds: u64,

    #[arg(long)]
    #[arg(help = "Failures in a row to determine if target is failing")]
    #[arg(default_value = "5")]
    failure_threshold: u8,
}

#[tokio::main]
#[tracing::instrument(level = "info")]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    let mut log_builder = log::Builder::new();
    if let Some(log_level) = cli.log_level {
        log_builder.with_level(log_level);
    }
    log_builder.build();
    // metric::register_metrics(cli.metrics_port);

    match cli.command {
        Commands::Run(args) => {
            run(args, cli.metrics_port).await?;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "info")]
async fn run(
    args: Run,
    metrics_port: Option<u16>,
) -> Result<(), Error> {
    let metrics = metric::MetricProvider::new();
    let background_threads: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>> =
        Arc::new(Mutex::new(Vec::new()));

    let mut background_threads_locked = background_threads
        .lock()
        .expect("Failed to remember our background threads");

    let targets = args.target;

    for target in targets {
        background_threads_locked.push(tokio::spawn(async move {
            let runner = runner::RunnerBuilder::new()
                .target(target)
                .connect_timeout_ms(args.connect_timeout_ms)
                .timeout_ms(args.timeout_ms)
                .wait_time_seconds(args.wait_time_seconds)
                .build();
            match runner.run().await {
                Err(e) => {
                    tracing::error!("handler error: {}", e);
                }
                _ => {}
            }
        }));
    }

    metrics.listen(metrics_port).await?;

    Ok(())
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{source}")]
    RunnerError {
        #[from]
        source: runner::Error,
    },

    #[error("{source}")]
    TokioError {
        #[from]
        source: tokio::task::JoinError,
    },

    #[error("{source}")]
    MetricsError {
        #[from]
        source: metric::Error,
    },
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_cli() {
        let cli = Cli::parse_from(&["netcheck", "run"]);
        assert_eq!(
            cli.command,
            Commands::Run(Run {
                target: vec![runner::Target::new(
                    "external".to_string(),
                    vec![
                        "https://one.one.one.one".parse().unwrap(),
                        "https://dns.google".parse().unwrap(),
                    ],
                ),],
                connect_timeout_ms: 500,
                timeout_ms: 500,
                wait_time_seconds: 2,
                failure_threshold: 5,
            })
        );
    }

    #[test]
    fn test_cli_with_args() {
        let cli = Cli::parse_from(&[
            "netcheck",
            "run",
            "--target",
            "internal=https://google.com,https://example.com",
            "--target",
            "external=https://example.com",
            "--connect",
            "1",
            "--timeout",
            "1",
            "--wait",
            "1",
            "--failure-threshold",
            "1",
        ]);
        assert_eq!(
            cli.command,
            Commands::Run(Run {
                target: vec![
                    runner::Target::new(
                        "internal".to_string(),
                        vec![
                            "https://google.com".parse().unwrap(),
                            "https://example.com".parse().unwrap(),
                        ],
                    ),
                    runner::Target::new(
                        "external".to_string(),
                        vec!["https://example.com".parse().unwrap()],
                    ),
                ],
                connect_timeout_ms: 1,
                timeout_ms: 1,
                wait_time_seconds: 1,
                failure_threshold: 1,
            })
        );
    }
}
