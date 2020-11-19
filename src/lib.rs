#![feature(duration_zero)]

//! # Vinted Elasticsearch exporter
#![deny(
    warnings,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    private_in_public,
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
    unused_extern_crates,
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
mod options;
pub use options::ExporterOptions;

mod metadata;

pub(crate) mod metrics;

const NAMESPACE: &'static str = "elasticsearch";

/// The default [`Histogram`] buckets for Elasticsearch.
pub const DEFAULT_BUCKETS: &[f64; 19] = &[
    0.020, 0.040, 0.060, 0.080, 0.1, // <= 100ms
    0.250, 0.500, 0.750, 1.0, // <= 1 second
    2.0, 4.0, 6.0, 8.0, 10.0, // <= 10 seconds
    20.0, 30.0, 40.0, 50.0, 60.0, // <= 1 minute
];

/// Labels type with ordered keys
pub type Labels = BTreeMap<String, String>;

/// Collection labels
pub type CollectionLabels = BTreeMap<String, Vec<String>>;

/// Exporter polling intervals
pub type ExporterPollIntervals = BTreeMap<String, Duration>;

/// Exporter metrics switch ON/OFF
pub type ExporterMetricsSwitch = BTreeMap<String, bool>;

/// Elasticsearch exporter
#[derive(Debug, Clone)]
pub struct Exporter(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    /// Elasticsearch client instance
    client: Elasticsearch,
    /// Exporter options
    options: ExporterOptions,
    /// Constant exporter labels, e.g.: cluster
    const_labels: HashMap<String, String>,

    /// Node ID to node name map for adding extra metadata labels
    /// {"U-WnGaTpRxucgde3miiDWw": "m1-supernode.example.com"}
    metadata: metadata::IdToMetadata,
}

impl Exporter {
    /// Elasticsearch client instance
    pub fn client(&self) -> &Elasticsearch {
        &self.0.client
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
    pub fn metadata(&self) -> &metadata::IdToMetadata {
        &self.0.metadata
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

        let metadata = metadata::build(&client).await?;
        let cluster_name = metadata::cluster_name(&client).await?;

        let mut const_labels = HashMap::new();
        let _ = const_labels.insert("cluster".into(), cluster_name);

        Ok(Self(Arc::new(Inner {
            client,
            options,
            const_labels,
            metadata,
        })))
    }

    /// Spawn collectors
    pub async fn spawn(self) {
        Self::spawn_cat(self.clone());
        Self::spawn_cluster(self.clone());
        Self::spawn_nodes(self.clone());
    }

    fn spawn_cluster(exporter: Self) {
        use metrics::_cluster::*;

        is_metric_enabled!(exporter, health);
    }

    fn spawn_nodes(exporter: Self) {
        use metrics::_nodes::*;

        is_metric_enabled!(exporter, usage);
        is_metric_enabled!(exporter, stats);
        is_metric_enabled!(exporter, info);
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
    fn spawn_cat(exporter: Self) {
        use metrics::_cat::*;

        is_metric_enabled!(exporter, allocation);
        is_metric_enabled!(exporter, shards);
        is_metric_enabled!(exporter, indices);
        is_metric_enabled!(exporter, segments);
        is_metric_enabled!(exporter, nodes);
        is_metric_enabled!(exporter, recovery);
        is_metric_enabled!(exporter, health);
        is_metric_enabled!(exporter, pending_tasks);
        is_metric_enabled!(exporter, aliases);
        is_metric_enabled!(exporter, thread_pool);
        is_metric_enabled!(exporter, plugins);
        is_metric_enabled!(exporter, fielddata);
        is_metric_enabled!(exporter, nodeattrs);
        is_metric_enabled!(exporter, repositories);
        is_metric_enabled!(exporter, templates);
        is_metric_enabled!(exporter, transforms);

        is_metric_enabled!(exporter, transforms);
    }
}

/// Convenience macro to poll metrics
#[macro_export]
macro_rules! is_metric_enabled {
    ($exporter:expr, $metric:ident) => {
        if $exporter.options().is_metric_enabled($metric::SUBSYSTEM) {
            let _ = tokio::spawn($metric::poll($exporter.clone()));
        }
    };
}
