#![feature(str_split_once)]

#[macro_use]
extern crate log;

use clap::Clap;
use hyper::{
    header::CONTENT_TYPE,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode,
};
use prometheus::{Encoder, TextEncoder, TEXT_FORMAT};
use std::convert::Infallible;
use std::env;
use std::panic;
use std::time::Duration;

use elasticsearch_exporter::{exporter_metrics::HTTP_REQ_HISTOGRAM, Exporter, ExporterOptions};

fn build_response(status: StatusCode, body: Body) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, TEXT_FORMAT)
        .body(body)
        .expect("valid Response built")
}

async fn serve_req(req: Request<Body>, options: String) -> Result<Response<Body>, Infallible> {
    let path = req.uri().path();

    let timer = HTTP_REQ_HISTOGRAM.with_label_values(&[path]).start_timer();

    let response = match path {
        "/health" | "/healthy" | "/healthz" => build_response(StatusCode::OK, Body::from("Ok")),
        "/" => build_response(StatusCode::OK, Body::from(options.to_string())),

        "/metrics" => {
            let encoder = TextEncoder::new();

            let mut buffer = vec![];
            match encoder.encode(&prometheus::gather(), &mut buffer) {
                Ok(_) => build_response(StatusCode::OK, Body::from(buffer)),
                Err(e) => {
                    error!("prometheus encoder err {}", e);

                    build_response(StatusCode::INTERNAL_SERVER_ERROR, Body::empty())
                }
            }
        }
        _ => build_response(
            StatusCode::NOT_FOUND,
            Body::from(format!("Path {} not found", path)),
        ),
    };

    timer.observe_duration();

    Ok(response)
}

/// Setup panic hook
pub fn panic_hook() {
    panic::set_hook(Box::new(|err| {
        eprintln!("Panic error {:?}, exiting program.", err);
        std::process::exit(70);
    }));
}

mod cli;
use cli::{signal_channel, Opts};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    panic_hook();

    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "info,elasticsearch_exporter=debug");
    }

    pretty_env_logger::init();
    let mut opts = Opts::parse();

    if let Ok(Ok(port)) = env::var("PORT").map(|p| p.parse::<u16>()) {
        opts.listen_addr.set_port(port);
    }

    let options = ExporterOptions {
        elasticsearch_url: opts.elasticsearch_url.clone(),
        elasticsearch_global_timeout: Duration::from_millis(opts.elasticsearch_global_timeout_ms),
        exporter_skip_labels: opts.exporter_skip_labels.0.clone(),
        exporter_skip_metrics: opts.exporter_skip_metrics.0.clone(),
        exporter_include_labels: opts.exporter_include_labels.0.clone(),
        exporter_poll_default_interval: Duration::from_millis(
            opts.exporter_poll_default_interval_ms,
        ),
        exporter_histogram_buckets: elasticsearch_exporter::DEFAULT_BUCKETS.to_vec(),
        exporter_skip_zero_metrics: !opts.exporter_skip_zero_metrics,
        exporter_poll_intervals: opts.exporter_poll_intervals.0.clone(),
        exporter_metrics_enabled: opts.exporter_metrics_enabled.0.clone(),
        exporter_metadata_refresh_interval: Duration::from_secs(
            opts.exporter_metadata_refresh_interval_seconds,
        ),
    };

    info!("{}", options);

    let options_clone = options.clone();
    let new_service = make_service_fn(move |socket: &AddrStream| {
        let options_string = options_clone.to_string();

        let svc = service_fn(move |req| serve_req(req, options_string.clone()));
        trace!("incoming socket request: {:?}", socket);
        async move { Ok::<_, Infallible>(svc) }
    });

    let signal_rx = signal_channel();

    match Exporter::new(options).await {
        Ok(exporter) => {
            let _ = tokio::spawn(exporter.spawn());
        }
        Err(e) => {
            error!("{}", e);

            std::process::exit(70);
        }
    }

    info!("Listening on http://{}", opts.listen_addr);

    Server::bind(&opts.listen_addr)
        // TCP
        .tcp_keepalive(Some(Duration::from_secs(opts.hyper_tcp_keepalive_sec)))
        .tcp_nodelay(true)
        // HTTP 1
        .http1_keepalive(true)
        .http1_half_close(false)
        .http1_max_buf_size(opts.hyper_http1_max_buf_size)
        // HTTP 2
        .http2_keep_alive_interval(Duration::from_secs(opts.hyper_tcp_keepalive_sec))
        .http2_keep_alive_timeout(Duration::from_secs(opts.hyper_http2_keep_alive_timeout_sec))
        .http2_adaptive_window(true)
        .serve(new_service)
        .with_graceful_shutdown(async move {
            signal_rx.await.ok();
            info!("Graceful context shutdown");
        })
        .await?;

    Ok(())
}
