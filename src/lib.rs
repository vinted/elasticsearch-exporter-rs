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
use elasticsearch::cluster::ClusterHealthParts;
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use elasticsearch::Elasticsearch;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

/// Generic collector of Elasticsearch metrics
pub mod collection;
/// Metric
pub mod metric;
mod options;
pub use options::ExporterOptions;

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

/// Elasticsearch exporter
#[derive(Debug, Clone)]
pub struct Exporter {
    /// Elasticsearch client instance
    pub client: Elasticsearch,
    /// Exporter options
    pub options: ExporterOptions,
    /// Constant exporter labels, e.g.: cluster
    pub const_labels: HashMap<String, String>,
}

impl Exporter {
    /// Spawn exporter
    pub async fn new(options: ExporterOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let connection_pool = SingleNodeConnectionPool::new(options.elasticsearch_url.clone());
        let transport = TransportBuilder::new(connection_pool)
            .timeout(options.elasticsearch_global_timeout)
            .build()?;

        let client = Elasticsearch::new(transport);
        info!("Elasticsearch::ping");
        let _ = client.ping().send().await?;

        let cluster_name = client
            .cluster()
            .health(ClusterHealthParts::None)
            .send()
            .await?
            .json::<Value>()
            .await?["cluster_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let mut const_labels = HashMap::new();
        let _ = const_labels.insert("cluster".into(), cluster_name);

        Ok(Self {
            client,
            options,
            const_labels,
        })
    }

    /// Spawn collectors
    pub async fn spawn(self) {
        info!("Spawned");
        Self::spawn_cat(self.clone());
        Self::spawn_cluster(self.clone());
    }

    fn spawn_cluster(exporter: Self) {
        let _ = tokio::spawn(metrics::_cluster::health::poll(exporter.clone()));
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
        let _ = tokio::spawn(metrics::_cat::allocation::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::shards::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::indices::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::segments::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::nodes::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::recovery::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::health::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::pending_tasks::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::aliases::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::thread_pool::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::plugins::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::fielddata::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::nodeattrs::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::repositories::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::templates::poll(exporter.clone()));
        let _ = tokio::spawn(metrics::_cat::transforms::poll(exporter));
    }
}
