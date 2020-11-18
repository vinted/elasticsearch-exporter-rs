use clap::Clap;
use std::error::Error as StdError;
use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::signal;
use tokio::sync::oneshot::{self, Receiver, Sender};
use url::Url;

use elasticsearch_exporter::{
    CollectionLabels, ExporterMetricsSwitch, ExporterPollIntervals, Labels,
};

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

#[derive(Debug)]
pub struct SimpleError(String);

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for SimpleError {}

const HASH_MAP_STR_FORMAT: &'static str = "cat_indices=id,pri,rep&cat_nodes=heap.percent,jdk";

#[derive(Clap, Clone, Debug)]
pub struct Opts {
    /// Application listen address
    #[clap(long = "listen_addr", default_value = "0.0.0.0:9222")]
    pub listen_addr: SocketAddr,

    /// HTTP max buffer size in KiB
    #[clap(long = "hyper_max_buffer_size", default_value = "1048576")] // 1MiB
    pub hyper_http1_max_buf_size: usize,

    /// TCP keepalive
    #[clap(long = "hyper_tcp_keepalive_sec", default_value = "30")]
    pub hyper_tcp_keepalive_sec: u64,

    /// HTTPS keepalive timeout
    #[clap(long = "hyper_http2_keep_alive_timeout_sec", default_value = "60")]
    pub hyper_http2_keep_alive_timeout_sec: u64,

    /// Elasticsearch URL, provide with protocol "https?://"
    #[clap(long = "elasticsearch_url", default_value = "http://127.0.0.1:9200")]
    pub elasticsearch_url: Url,

    /// Elasticsearch global timeout of all metrics
    #[clap(long = "elasticsearch_global_timeout_ms", default_value = "30000")]
    pub elasticsearch_global_timeout_ms: u64,

    /// Exporter skip labels
    #[clap(
        long = "exporter_skip_labels",
        default_value = "cat_allocation=health,status&cat_fielddata=id&cat_indices=health,status&cat_nodeattrs=id&cat_nodes=health,status,pid&cat_plugins=id,description&cat_segments=health,status,checkpoint,prirep&cat_shards=health,status,checkpoint,prirep&cat_templates=composed_of&cat_thread_pool=node_id,ephemeral_node_id,pid&cat_transforms=health,status&cluster_stats=segment,patterns"
    )]
    pub exporter_skip_labels: HashMapVec,

    /// Exporter include labels
    #[clap(
        long = "exporter_include_labels",
        default_value = "cat_health=shards&cat_aliases=index,alias&cat_allocation=node&cat_fielddata=node,field&cat_indices=index&cat_nodeattrs=node,attr&cat_nodes=index,name,node_role&cat_pending_tasks=index&cat_plugins=name&cat_recovery=index,shard,stage,type&cat_repositories=index&cat_segments=index,shard&cat_shards=index,node,shard&cat_templates=name,index_patterns&cat_thread_pool=node_name,name,type&cat_transforms=index&cluster_health=status&nodes_usage=name&nodes_stats=name"
    )]
    pub exporter_include_labels: HashMapVec,

    /// Exporter skip labels
    #[clap(
        long = "exporter_skip_metrics",
        default_value = "cat_aliases=filter,routing_index,routing_search,is_write_index&cat_nodeattrs=pid&cat_recovery=start_time,start_time_millis,stop_time,stop_time_millis&cat_templates=order&nodes_usage=_nodes_total,_nodes_successful,since"
    )]
    pub exporter_skip_metrics: HashMapVec,

    /// Exporter default polling interval in milliseconds
    #[clap(long = "exporter_poll_default_interval_ms", default_value = "5000")]
    pub exporter_poll_default_interval_ms: u64,

    /// Exporter skip zero metrics, prevents exposing metrics that are empty
    /// this greatly reduces response size
    #[clap(long = "exporter_skip_zero_metrics")]
    pub exporter_skip_zero_metrics: bool,

    /// Exporter custom poll intervals for metrics in case custom interval is not
    /// defined it will fall back to default polling interval
    #[clap(long = "exporter_poll_intervals", default_value = "cluster_health=5s")]
    pub exporter_poll_intervals: HashMapDuration,

    /// Exporter metrics switch defined which metrics are turned ON
    #[clap(
        long = "exporter_metrics_switch",
        default_value = "cat_health=true&cat_indices=true&nodes_stats=true"
    )]
    pub exporter_metrics_switch: HashMapSwitch,
}

#[derive(Debug, Clone, Default)]
pub struct HashMapSwitch(pub ExporterMetricsSwitch);

impl FromStr for HashMapSwitch {
    type Err = SimpleError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Self(serde_qs::from_str(input).map_err(|e| {
            SimpleError(format!(
                "Usage `cat_health=true&cat_templates=false`, you provided `{}`",
                e
            ))
        })?))
    }
}

#[derive(Debug, Clone, Default)]
pub struct HashMapDuration(pub ExporterPollIntervals);

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
pub struct HashMapVec(pub CollectionLabels);

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
struct HashMapStr(pub Labels);

impl FromStr for HashMapStr {
    type Err = SimpleError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Self(serde_qs::from_str(input).map_err(|e| {
            SimpleError(format!(
                "Usage `{}`, you provided `{}`",
                HASH_MAP_STR_FORMAT, e
            ))
        })?))
    }
}
