#![feature(str_split_once)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate prometheus;
#[macro_use]
extern crate log;

use clap::Clap;
use hyper::server::conn::AddrStream;
use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode,
};
use prometheus::{Encoder, HistogramVec, TextEncoder, TEXT_FORMAT};
use std::convert::Infallible;
use std::env;
use std::error::Error as StdError;
use std::fmt;
use std::net::SocketAddr;
use std::panic;
use std::str::FromStr;
use std::time::Duration;
use tokio::signal;
use tokio::sync::oneshot::{self, Receiver, Sender};
use url::Url;

use elasticsearch_exporter::{
    CollectionLabels, Exporter, ExporterMetricsSwitch, ExporterOptions, ExporterPollIntervals,
    Labels,
};

lazy_static! {
    static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["handler"]
    )
    .expect("valid histogram vec metric");
}

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

#[derive(Clap, Clone, Debug)]
struct Opts {
    /// Application listen address
    #[clap(long = "listen_addr", default_value = "0.0.0.0:9222")]
    listen_addr: SocketAddr,

    /// HTTP max buffer size in KiB
    #[clap(long = "hyper_max_buffer_size", default_value = "1048576")] // 1MiB
    hyper_http1_max_buf_size: usize,

    /// TCP keepalive
    #[clap(long = "hyper_tcp_keepalive_sec", default_value = "30")]
    hyper_tcp_keepalive_sec: u64,

    /// HTTPS keepalive timeout
    #[clap(long = "hyper_http2_keep_alive_timeout_sec", default_value = "60")]
    hyper_http2_keep_alive_timeout_sec: u64,

    #[clap(long = "elasticsearch_url", default_value = "http://127.0.0.1:9200")]
    elasticsearch_url: Url,

    #[clap(long = "elasticsearch_global_timeout_ms", default_value = "30000")]
    elasticsearch_global_timeout_ms: u64,

    #[clap(long = "elasticsearch_cat_headers", default_value = "cat_nodes=*")]
    elasticsearch_cat_headers: HashMapStr,

    #[clap(
        long = "elasticsearch_skip_labels",
        default_value = "cat_allocation=health,status&cat_fielddata=id&cat_indices=health,status&cat_nodeattrs=id&cat_nodes=health,status,pid&cat_plugins=id,description&cat_segments=health,status,checkpoint,prirep&cat_shards=health,status,checkpoint,prirep&cat_templates=composed_of&cat_thread_pool=node_id,ephemeral_node_id,pid&cat_transforms=health,status&cluster_stats=segment,patterns"
    )]
    elasticsearch_skip_labels: HashMapVec,

    #[clap(
        long = "elasticsearch_include_labels",
        default_value = "cat_health=shards&cat_aliases=index,alias&cat_allocation=node&cat_fielddata=node,field&cat_indices=index&cat_nodeattrs=node,attr&cat_nodes=index,name,node_role&cat_pending_tasks=index&cat_plugins=name&cat_recovery=index,shard,stage,type&cat_repositories=index&cat_segments=index,shard&cat_shards=index,node,shard&cat_templates=name,index_patterns&cat_thread_pool=node_name,name,type&cat_transforms=index&cluster_health=status&nodes_usage=name,usage"
    )]
    elasticsearch_include_labels: HashMapVec,

    #[clap(
        long = "elasticsearch_skip_metrics",
        default_value = "cat_health=epoch,timestamp&cat_aliases=filter,routing_index,routing_search,is_write_index&cat_nodeattrs=pid&cat_recovery=start_time,start_time_millis,stop_time,stop_time_millis&cat_templates=order&nodes_usage=_nodes_total,_nodes_successful,timestamp,since"
    )]
    elasticsearch_skip_metrics: HashMapVec,

    #[clap(long = "exporter_poll_default_interval_ms", default_value = "5000")]
    exporter_poll_default_interval_ms: u64,

    #[clap(long = "exporter_skip_zero_metrics")]
    exporter_skip_zero_metrics: bool,

    #[clap(long = "exporter_poll_intervals", default_value = "cluster_health=5s")]
    exporter_poll_intervals: HashMapDuration,

    #[clap(
        long = "exporter_metrics_switch",
        default_value = "cat_health=true&cat_indices=true"
    )]
    exporter_metrics_switch: HashMapSwitch,
}

#[derive(Debug, Clone, Default)]
struct HashMapSwitch(ExporterMetricsSwitch);

impl FromStr for HashMapSwitch {
    type Err = SimpleError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut map = ExporterMetricsSwitch::new();

        let parts = input.trim().split("&").into_iter().collect::<Vec<&str>>();

        for part in parts.into_iter() {
            match part.split_once("=") {
                Some((key, value)) => {
                    let bool_value = if value == "true" { true } else { false };

                    let _ = map.insert(key.to_string(), bool_value);
                }
                None => {
                    return Err(SimpleError(format!(
                        "Usage `cat_health=true&cat_templates=false`, you provided `{}`",
                        part
                    )))
                }
            }
        }

        Ok(Self(map))
    }
}

#[derive(Debug, Clone, Default)]
struct HashMapDuration(ExporterPollIntervals);

impl FromStr for HashMapDuration {
    type Err = SimpleError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut map = ExporterPollIntervals::new();

        let parts = input.trim().split("&").into_iter().collect::<Vec<&str>>();

        for part in parts.into_iter() {
            match part.split_once("=") {
                Some((key, value)) => match value.parse::<humantime::Duration>() {
                    Ok(time) => {
                        let _ = map.insert(key.to_string(), *time);
                    }
                    Err(e) => {
                        return Err(SimpleError(format!(
                            "Failed to parse time for key {} value {} err {}",
                            key, value, e
                        )))
                    }
                },
                None => {
                    return Err(SimpleError(format!(
                        "Usage `{}`, you provided `{}`",
                        HASH_MAP_STR_FORMAT, part
                    )))
                }
            }
        }

        Ok(Self(map))
    }
}

#[derive(Clone, Debug, Default)]
struct HashMapVec(CollectionLabels);

impl FromStr for HashMapVec {
    type Err = SimpleError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut map = CollectionLabels::new();

        let parts = input.trim().split("&").into_iter().collect::<Vec<&str>>();

        for part in parts.into_iter() {
            match part.split_once("=") {
                Some((key, value)) => {
                    let labels = value
                        .split(",")
                        .map(|value| value.to_string())
                        .collect::<Vec<String>>();

                    if let Some(values) = map.get_mut(key) {
                        values.extend(labels);
                    } else {
                        let _ = map.insert(key.to_string(), labels);
                    }
                }
                None => {
                    return Err(SimpleError(format!(
                        "Usage `{}`, you provided `{}`",
                        HASH_MAP_STR_FORMAT, part
                    )))
                }
            }
        }

        Ok(Self(map))
    }
}

#[derive(Clone, Debug, Default)]
struct HashMapStr(Labels);

#[derive(Debug)]
pub struct SimpleError(String);

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for SimpleError {}

const HASH_MAP_STR_FORMAT: &'static str = "cat_indices=id,pri,rep&cat_nodes=heap.percent,jdk";

impl FromStr for HashMapStr {
    type Err = SimpleError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut map = Labels::new();

        let parts = input.trim().split("&").into_iter().collect::<Vec<&str>>();

        for part in parts.into_iter() {
            match part.split_once("=") {
                Some((key, value)) => {
                    let _ = map.insert(key.to_string(), value.to_string());
                }
                None => {
                    return Err(SimpleError(format!(
                        "Usage `{}`, you provided `{}`",
                        HASH_MAP_STR_FORMAT, part
                    )))
                }
            }
        }

        Ok(Self(map))
    }
}

pub fn unit_channel() -> (Sender<()>, Receiver<()>) {
    oneshot::channel()
}

/// Wait for SIGINT signal
async fn wait_for_signal(tx: Sender<()>) {
    let _ = signal::ctrl_c().await;
    info!("SIGINT received: shutting down");
    let _ = tx.send(());
}

pub fn signal_channel() -> Receiver<()> {
    let (signal_tx, signal_rx) = unit_channel();

    let _ = tokio::spawn(wait_for_signal(signal_tx));

    signal_rx
}

/// Setup panic hook
pub fn panic_hook() {
    panic::set_hook(Box::new(|err| {
        eprintln!("Panic error {:?}, exiting program.", err);
        std::process::exit(70);
    }));
}

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
        elasticsearch_skip_labels: opts.elasticsearch_skip_labels.0.clone(),
        elasticsearch_skip_metrics: opts.elasticsearch_skip_metrics.0.clone(),
        elasticsearch_include_labels: opts.elasticsearch_include_labels.0.clone(),
        elasticsearch_cat_headers: opts.elasticsearch_cat_headers.0.clone(),
        exporter_poll_default_interval: Duration::from_millis(
            opts.exporter_poll_default_interval_ms,
        ),
        exporter_histogram_buckets: elasticsearch_exporter::DEFAULT_BUCKETS.to_vec(),
        exporter_skip_zero_metrics: !opts.exporter_skip_zero_metrics,
        exporter_poll_intervals: opts.exporter_poll_intervals.0.clone(),
        exporter_metrics_switch: opts.exporter_metrics_switch.0.clone(),
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
