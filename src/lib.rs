#![feature(hash_extract_if)]

//! # Vinted Elasticsearch exporter
#![deny(
    warnings,
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_bounds,
    private_interfaces,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    trivial_numeric_casts,
    unreachable_pub,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    deprecated,
    unconditional_recursion,
    unknown_lints,
    unreachable_code,
    unused_mut
)]

#[macro_use]
extern crate prometheus;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use elasticsearch::Elasticsearch;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

/// Generic collector of Elasticsearch metrics
pub mod collection;
/// Metric
pub mod metric;

/// Exporter metrics
mod exporter_metrics;

mod options;
pub use options::ExporterOptions;

/// Reserved labels
pub mod reserved;

/// Cluster metadata
pub mod metadata;

pub(crate) mod metrics;

const NAMESPACE: &str = "elasticsearch";

/// Labels type with ordered keys
pub type Labels = BTreeMap<String, String>;

/// Collection labels
pub type CollectionLabels = BTreeMap<String, Vec<String>>;

/// Exporter polling intervals
pub type ExporterPollIntervals = HashMap<String, Duration>;

/// Exporter metrics switch ON/OFF
pub type ExporterMetricsSwitch = BTreeMap<String, bool>;

/// Elasticsearch exporter
#[derive(Debug, Clone)]
pub struct Exporter(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    /// Name of Elasticsearch cluster exporter is working
    cluster_name: String,
    /// Elasticsearch client instance
    client: Elasticsearch,
    /// Exporter options
    options: ExporterOptions,
    /// Constant exporter labels, e.g.: cluster
    const_labels: HashMap<String, String>,

    /// Node ID to node name map for adding extra metadata labels
    /// {"U-WnGaTpRxucgde3miiDWw": "m1-supernode.example.com"}
    nodes_metadata: metadata::IdToMetadata,
}

impl Exporter {
    /// Elasticsearch client instance
    pub fn client(&self) -> &Elasticsearch {
        &self.0.client
    }

    /// Elasticsearch cluster name
    pub fn cluster_name(&self) -> &str {
        &self.0.cluster_name
    }

    /// Exporter options
    pub fn options(&self) -> &ExporterOptions {
        &self.0.options
    }

    /// Exporter options
    pub fn const_labels(&self) -> HashMap<String, String> {
        self.0.const_labels.clone()
    }

    /// Node ID to node name map for adding extra metadata labels
    /// {"U-WnGaTpRxucgde3miiDWw": "m1-supernode.example.com"}
    pub fn nodes_metadata(&self) -> &metadata::IdToMetadata {
        &self.0.nodes_metadata
    }

    /// Spawn exporter
    pub async fn new(options: ExporterOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let connection_pool = SingleNodeConnectionPool::new(options.elasticsearch_url.clone());
        let transport = TransportBuilder::new(connection_pool)
            .timeout(options.elasticsearch_global_timeout)
            .build()?;

        let client = Elasticsearch::new(transport);
        info!("Elasticsearch: ping");
        let _ = client.ping().send().await?;

        let nodes_metadata = if options.enable_metadata_refresh() {
            metadata::node_data::build(&client).await?
        } else {
            info!("Skip metadata refresh");
            // This will generate empty map
            Default::default()
        };

        let cluster_name = metadata::cluster_name(&client).await?;

        let mut const_labels = HashMap::new();
        let _ = const_labels.insert("cluster".into(), cluster_name.clone());

        Ok(Self(Arc::new(Inner {
            cluster_name,
            client,
            options,
            const_labels,
            nodes_metadata,
        })))
    }

    /// Spawn collectors
    pub async fn spawn(self) {
        self.spawn_cat();
        self.spawn_cluster();
        self.spawn_nodes();
        self.spawn_stats();

        if self.options().enable_metadata_refresh() {
            #[allow(clippy::let_underscore_future)]
            let _ = tokio::spawn(metadata::node_data::poll(self));
        }
    }

    fn spawn_cluster(&self) {
        use metrics::_cluster::*;

        is_metric_enabled!(self.clone(), health);
    }

    fn spawn_stats(&self) {
        use metrics::_stats::*;

        is_metric_enabled!(self.clone(), _all);
    }

    fn spawn_nodes(&self) {
        use metrics::_nodes::*;

        is_metric_enabled!(self.clone(), usage);
        is_metric_enabled!(self.clone(), stats);
        is_metric_enabled!(self.clone(), info);
    }

    // =^.^=
    // /_cat/allocation
    // /_cat/shards
    // /_cat/indices
    // /_cat/segments
    // /_cat/nodes
    // /_cat/recovery
    // /_cat/health
    // /_cat/pending_tasks
    // /_cat/aliases
    // /_cat/thread_pool
    // /_cat/plugins
    // /_cat/fielddata
    // /_cat/nodeattrs
    // /_cat/repositories
    // /_cat/templates
    // /_cat/transforms
    fn spawn_cat(&self) {
        use metrics::_cat::*;

        is_metric_enabled!(self.clone(), allocation);
        is_metric_enabled!(self.clone(), shards);
        is_metric_enabled!(self.clone(), indices);
        is_metric_enabled!(self.clone(), segments);
        is_metric_enabled!(self.clone(), nodes);
        is_metric_enabled!(self.clone(), recovery);
        is_metric_enabled!(self.clone(), health);
        is_metric_enabled!(self.clone(), pending_tasks);
        is_metric_enabled!(self.clone(), aliases);
        is_metric_enabled!(self.clone(), thread_pool);
        is_metric_enabled!(self.clone(), plugins);
        is_metric_enabled!(self.clone(), fielddata);
        is_metric_enabled!(self.clone(), nodeattrs);
        is_metric_enabled!(self.clone(), repositories);
        is_metric_enabled!(self.clone(), templates);
        is_metric_enabled!(self.clone(), transforms);
    }

    pub(crate) fn random_delay() -> u64 {
        oorandom::Rand64::new(292).rand_range(150..800)
    }
}

/// Convenience macro to poll metrics
#[macro_export]
macro_rules! is_metric_enabled {
    ($exporter:expr, $metric:ident) => {
        if $exporter.options().is_metric_enabled($metric::SUBSYSTEM) {
            #[allow(clippy::let_underscore_future)]
            let _ = tokio::spawn($metric::poll($exporter.clone()));
        }
    };
}
