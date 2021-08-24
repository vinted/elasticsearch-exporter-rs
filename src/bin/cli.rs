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

const HASH_MAP_STR_FORMAT: &str = "cat_indices=id,pri,rep&cat_nodes=heap.percent,jdk";

#[derive(Clap, Clone, Debug)]
pub struct Opts {
    /// Application listen address
    #[clap(long = "listen_addr", default_value = "0.0.0.0:9222")]
    pub listen_addr: SocketAddr,

    /// HTTP max buffer size in KiB
    #[clap(long = "hyper_max_buffer_size", default_value = "1048576")] // 1MiB
    pub hyper_http1_max_buf_size: usize,

    /// TCP keepalive
    #[clap(long = "hyper_tcp_keepalive", default_value = "30s")]
    pub hyper_tcp_keepalive: humantime::Duration,

    /// HTTPS keepalive timeout
    #[clap(long = "hyper_http2_keep_alive_timeout", default_value = "1m")]
    pub hyper_http2_keep_alive_timeout: humantime::Duration,

    /// Elasticsearch URL, provide with protocol "https?://"
    #[clap(long = "elasticsearch_url", default_value = "http://127.0.0.1:9200")]
    pub elasticsearch_url: Url,

    /// Elasticsearch global timeout of all metrics
    #[clap(long = "elasticsearch_global_timeout", default_value = "30s")]
    pub elasticsearch_global_timeout: humantime::Duration,

    /// Exporter timeout for subsystems, in case subsystem timeout is not defined
    /// default global timeout is used
    #[clap(
        long = "elasticsearch_subsystem_timeouts",
        default_value = "nodes_stats=15s"
    )]
    pub elasticsearch_subsystem_timeouts: HashMapDuration,

    /// Elasticsearch path parameters
    /// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-nodes-info.html#cluster-nodes-info-api-path-params
    #[clap(
        long = "elasticsearch_path_parameters",
        default_value = "nodes_info=http,jvm,thread_pool&nodes_stats=breaker,indices,jvm,os,process,transport,thread_pool"
    )]
    pub elasticsearch_path_parameters: HashMapVec,

    /// Exporter skip labels
    #[clap(
        long = "exporter_skip_labels",
        default_value = "cat_allocation=health,status&cat_fielddata=id&cat_indices=health,status&cat_nodeattrs=id&cat_nodes=health,status,pid&cat_plugins=id,description&cat_segments=health,status,checkpoint,prirep&cat_shards=health,status,checkpoint,prirep&cat_templates=composed_of&cat_thread_pool=node_id,ephemeral_node_id,pid&cat_transforms=health,status&cluster_stats=segment,patterns"
    )]
    pub exporter_skip_labels: HashMapVec,

    /// Exporter include labels
    #[clap(
        long = "exporter_include_labels",
        default_value = "cat_health=shards&cat_aliases=index,alias&cat_allocation=node&cat_fielddata=node,field&cat_indices=index&cat_nodeattrs=node,attr&cat_nodes=ip,name,node_role&cat_pending_tasks=index&cat_plugins=name&cat_recovery=index,shard,stage,type&cat_repositories=index&cat_segments=index,shard&cat_shards=index,node,shard&cat_templates=name,index_patterns&cat_thread_pool=node_name,name,type&cat_transforms=index&cluster_health=status&nodes_usage=name&nodes_stats=name,vin_cluster_version&nodes_info=name&stats=index"
    )]
    pub exporter_include_labels: HashMapVec,

    /// Exporter skip labels
    #[clap(
        long = "exporter_skip_metrics",
        default_value = "cat_aliases=filter,routing_index,routing_search,is_write_index&cat_nodeattrs=pid&cat_recovery=start_time,start_time_millis,stop_time,stop_time_millis&cat_templates=order&nodes_usage=_nodes_total,_nodes_successful,since"
    )]
    pub exporter_skip_metrics: HashMapVec,

    /// Exporter default polling interval in seconds
    #[clap(long = "exporter_poll_default_interval", default_value = "15s")]
    pub exporter_poll_default_interval: humantime::Duration,

    /// Exporter allow zero metrics, controls export of zero/empty  metrics
    #[clap(long = "exporter_allow_zero_metrics")]
    pub exporter_allow_zero_metrics: bool,

    /// Exporter custom poll intervals for metrics in case custom interval is not
    /// defined it will fall back to default polling interval
    #[clap(long = "exporter_poll_intervals", default_value = "cluster_health=5s")]
    pub exporter_poll_intervals: HashMapDuration,

    /// Exporter metrics switch defined which metrics are turned ON
    #[clap(
        long = "exporter_metrics_enabled",
        default_value = "cat_health=true&cat_indices=true&nodes_stats=true&stats=true"
    )]
    pub exporter_metrics_enabled: HashMapSwitch,

    /// Exporter metadata refresh interval
    #[clap(long = "exporter_metadata_refresh_interval", default_value = "3m")]
    pub exporter_metadata_refresh_interval: humantime::Duration,

    /// Elasticsearch query ?fields= for /_nodes/stats fields comma-separated list or
    /// wildcard expressions of fields to include in the statistics.
    #[clap(long = "elasticsearch_query_fields", default_value = "nodes_stats=*")]
    pub elasticsearch_query_fields: HashMapVec,

    /// Exporter default metrics lifeimte interval in seconds
    #[clap(
        long = "exporter_metrics_lifetime_default_interval",
        default_value = "15s"
    )]
    pub exporter_metrics_lifetime_default_interval: humantime::Duration,

    /// Exporter custom metrics lifetime intervals for defining how long
    /// metrics should be kept in memory, in case custom interval is not
    /// defined it will fall back to default polling interval
    #[clap(
        long = "exporter_metrics_lifetime_interval",
        default_value = "cat_indices=180s&cat_nodes=60s&cat_recovery=60s"
    )]
    pub exporter_metrics_lifetime_interval: HashMapDuration,
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

        let parts = input.trim().split('&').collect::<Vec<&str>>();

        for part in parts {
            match part.split_once('=') {
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

        let parts = input.trim().split('&').collect::<Vec<&str>>();

        for part in parts {
            match part.split_once('=') {
                Some((key, value)) => {
                    let labels = value
                        .split(',')
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
