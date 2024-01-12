// Copyright 2023 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! logging stuffs, inspired by databend
use std::env;
use std::sync::{Arc, Mutex, Once};

use once_cell::sync::Lazy;
use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use serde::{Deserialize, Serialize};
pub use tracing::{event, span, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter, EnvFilter, Registry};

pub use crate::{debug, error, info, log, trace, warn};

static GLOBAL_LOG_GUARD: Lazy<Arc<Mutex<Option<Vec<WorkerGuard>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

const DEFAULT_LOG_TARGETS: &str = "info";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingOptions {
    pub dir: Option<String>,
    pub level: Option<String>,
    pub enable_jaeger_tracing: bool,
}

impl Default for LoggingOptions {
    fn default() -> Self {
        Self {
            dir: None,
            level: None,
            enable_jaeger_tracing: false,
        }
    }
}

// #[derive(Default)]
pub struct TracingOptions {
    #[cfg(feature = "tokio-console")]
    pub tokio_console_addr: Option<String>,
}

impl Default for TracingOptions {
    fn default() -> Self {
        Self {
            #[cfg(feature = "tokio-console")]
            tokio_console_addr: Some("0.0.0.0:6669".to_owned()),
        }
    }
}

pub fn init_default_logging(app_name: &str) {
    static START: Once = Once::new();

    START.call_once(|| {
        let mut g = GLOBAL_LOG_GUARD.as_ref().lock().unwrap();

        let mut path = if let Ok(path) = std::env::current_exe() {
            path.parent().unwrap().to_owned()
        } else {
            std::env::temp_dir()
        };
        path.push("logs");
        let dir = path
            .file_name()
            .map(|p| p.to_str().unwrap_or(""))
            .unwrap_or("")
            .to_owned();
        let level = std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".to_string());

        let opts = LoggingOptions {
            dir: Some(dir.clone()),
            level: Some(level.clone()),
            ..Default::default()
        };
        *g = Some(init_global_logging(
            app_name,
            &opts,
            TracingOptions::default(),
        ));

        info!("logs dir = {}", dir);
        info!("logs tmpdir = {}", std::env::temp_dir().display());
    });
}

pub fn init_logging(
    app_name: &str,
    opts: &LoggingOptions,
) {
    static START: Once = Once::new();

    START.call_once(|| {
        let mut g = GLOBAL_LOG_GUARD.as_ref().lock().unwrap();

        *g = Some(init_global_logging(
            app_name,
            &opts,
            TracingOptions::default(),
        ));
    });
}

#[allow(clippy::print_stdout)]
pub fn init_global_logging(
    app_name: &str,
    opts: &LoggingOptions,
    tracing_opts: TracingOptions,
) -> Vec<WorkerGuard> {
    let mut guards = vec![];
    let binding = std::env::current_exe()
        .unwrap_or(std::env::temp_dir())
        .parent().unwrap()
        .display()
        .to_string();
    let dir = opts.dir.as_ref().unwrap_or(&binding);
    let level = &opts.level;
    let enable_jaeger_tracing = opts.enable_jaeger_tracing;

    // Enable log compatible layer to convert log record to tracing span.
    LogTracer::init().expect("log tracer must be valid");

    // Stdout layer.
    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let stdout_logging_layer = 
        Layer::new()
            .with_writer(stdout_writer)
            .with_target(false)
            .without_time();
    guards.push(stdout_guard);

    // JSON log layer.
    let rolling_appender = 
        RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix(app_name)
            .filename_suffix("log")
            .max_log_files(5)
            .build(dir)
            .expect("initializing rolling file appender failed");
    let (rolling_writer, rolling_writer_guard) = 
        tracing_appender::non_blocking(rolling_appender);
    let file_logging_layer = 
        Layer::new().with_writer(rolling_writer).with_ansi(false);
    // let file_logging_layer = 
    //     BunyanFormattingLayer::new(app_name.to_string(), rolling_writer);
    guards.push(rolling_writer_guard);

    // resolve log level settings from:
    // - options from command line or config files
    // - environment variable: RUST_LOG
    // - default settings
    let rust_log_env = std::env::var(EnvFilter::DEFAULT_ENV).ok();
    let targets_string = level
        .as_deref()
        .or(rust_log_env.as_deref())
        .unwrap_or(DEFAULT_LOG_TARGETS);
    let filter = targets_string
        .parse::<filter::Targets>()
        .expect("error parsing log level string");

    // Must enable 'tokio_unstable' cfg to use this feature.
    // For example: `RUSTFLAGS="--cfg tokio_unstable" cargo run -F common-telemetry/console -- standalone start`
    #[cfg(feature = "tokio-console")]
    let subscriber = {
        let tokio_console_layer = if let Some(tokio_console_addr) = &tracing_opts.tokio_console_addr
        {
            let addr: std::net::SocketAddr = tokio_console_addr.parse().unwrap_or_else(|e| {
                panic!("Invalid binding address '{tokio_console_addr}' for tokio-console: {e}");
            });
            println!("tokio-console listening on {addr}");

            Some(
                console_subscriber::ConsoleLayer::builder()
                    .server_addr(addr)
                    .spawn(),
            )
        } else {
            None
        };

        let stdout_logging_layer = stdout_logging_layer.with_filter(filter.clone());

        let file_logging_layer = file_logging_layer.with_filter(filter);

        Registry::default()
            .with(tokio_console_layer)
            .with(JsonStorageLayer)
            .with(stdout_logging_layer)
            .with(file_logging_layer)
            .with(err_file_logging_layer.with_filter(filter::LevelFilter::ERROR))
    };

    // consume the `tracing_opts`, to avoid "unused" warnings
    let _ = tracing_opts;

    #[cfg(not(feature = "tokio-console"))]
    let subscriber = Registry::default()
        .with(filter)
        .with(JsonStorageLayer)
        .with(stdout_logging_layer)
        .with(file_logging_layer);
        

    if enable_jaeger_tracing {
        // Jaeger layer.
        global::set_text_map_propagator(TraceContextPropagator::new());
        let tracer = opentelemetry_jaeger::new_agent_pipeline()
            .with_service_name(app_name)
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("install");
        let jaeger_layer = Some(tracing_opentelemetry::layer().with_tracer(tracer));
        let subscriber = subscriber.with(jaeger_layer);
        tracing::subscriber::set_global_default(subscriber)
            .expect("error setting global tracing subscriber");
    } else {
        tracing::subscriber::set_global_default(subscriber)
            .expect("error setting global tracing subscriber");
    }

    guards
}
