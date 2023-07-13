use std::fmt;
use std::time::Duration;
use url::Url;

use crate::{metrics, CollectionLabels, ExporterMetricsSwitch, ExporterPollIntervals};

/// Elasticsearch exporter options
#[derive(Debug, Clone)]
pub struct ExporterOptions {
    /// Elasticsearch cluster url
    pub elasticsearch_url: Url,
    /// Global HTTP request timeout
    pub elasticsearch_global_timeout: Duration,
    /// Elasticsearch /_nodes/stats fields comma-separated list or
    /// wildcard expressions of fields to include in the statistics.
    pub elasticsearch_query_fields: CollectionLabels,
    /// Elasticsearch /stats filter_path. Comma-separated list or
    /// wildcard expressions of paths to include in the statistics.
    pub elasticsearch_query_filter_path: CollectionLabels,
    /// Exporter timeout for subsystems, in case subsystem timeout is not defined
    /// default global timeout is used
    pub elasticsearch_subsystem_timeouts: ExporterPollIntervals,
    /// Elasticsearch path parameters
    /// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-nodes-info.html#cluster-nodes-info-api-path-params
    pub elasticsearch_path_parameters: CollectionLabels,

    //
    // Exporter
    //
    /// Exporter labels to skip
    pub exporter_skip_labels: CollectionLabels,
    /// Exporter labels to include, caution this may increase metric cardinality
    pub exporter_include_labels: CollectionLabels,
    /// Exporter labels to skip completely such as segment "id"
    pub exporter_skip_metrics: CollectionLabels,
    /// Exporter skip zero metrics
    pub exporter_skip_zero_metrics: bool,
    /// Exporter metrics switch either ON or OFF
    pub exporter_metrics_enabled: ExporterMetricsSwitch,
    /// Exporter metadata refresh interval
    pub exporter_metadata_refresh_interval: Duration,

    /// Metrics polling interval
    pub exporter_poll_default_interval: Duration,
    /// Exporter skip zero metrics
    pub exporter_poll_intervals: ExporterPollIntervals,

    /// Exporter metrics lifetime interval
    pub exporter_metrics_lifetime_interval: ExporterPollIntervals,
    /// Metrics metrics lifetime
    pub exporter_metrics_lifetime_default_interval: Duration,
}

impl ExporterOptions {
    /// Enable metadata refresh?
    pub(crate) fn enable_metadata_refresh(&self) -> bool {
        let cluster_subsystems = Self::nodes_subsystems();

        self.exporter_metrics_enabled
            .iter()
            .any(|(k, v)| cluster_subsystems.contains(&k.as_str()) && *v)
    }

    /// Check if metric is enabled
    pub fn is_metric_enabled(&self, subsystem: &'static str) -> bool {
        self.exporter_metrics_enabled.contains_key(subsystem)
    }

    /// ?fields= parameters for subsystems
    pub fn query_fields_for_subsystem(&self, subsystem: &'static str) -> Vec<&str> {
        self.elasticsearch_query_fields
            .get(subsystem)
            .map(|params| params.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
            .unwrap_or_default()
    }

    /// ?filter_path= parameters for subsystems
    pub fn query_filter_path_for_subsystem(&self, subsystem: &'static str) -> Vec<&str> {
        self.elasticsearch_query_filter_path
            .get(subsystem)
            .map(|params| params.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
            .unwrap_or_default()
    }

    /// Path parameters for subsystems
    pub fn path_parameters_for_subsystem(&self, subsystem: &'static str) -> Vec<&str> {
        self.elasticsearch_path_parameters
            .get(subsystem)
            .map(|params| params.iter().map(AsRef::as_ref).collect::<Vec<&str>>())
            .unwrap_or_default()
    }

    /// Get timeout for subsystem or fallback to global
    pub fn timeout_for_subsystem(&self, subsystem: &'static str) -> Duration {
        *self
            .elasticsearch_subsystem_timeouts
            .get(subsystem)
            .unwrap_or(&self.elasticsearch_global_timeout)
    }

    /// /_cat subsystems
    pub fn cat_subsystems() -> &'static [&'static str] {
        use metrics::_cat::*;

        &[
            allocation::SUBSYSTEM,
            shards::SUBSYSTEM,
            indices::SUBSYSTEM,
            segments::SUBSYSTEM,
            nodes::SUBSYSTEM,
            recovery::SUBSYSTEM,
            health::SUBSYSTEM,
            pending_tasks::SUBSYSTEM,
            aliases::SUBSYSTEM,
            thread_pool::SUBSYSTEM,
            plugins::SUBSYSTEM,
            fielddata::SUBSYSTEM,
            nodeattrs::SUBSYSTEM,
            repositories::SUBSYSTEM,
            templates::SUBSYSTEM,
            transforms::SUBSYSTEM,
        ]
    }

    /// /_cluster subsystems
    pub fn cluster_subsystems() -> &'static [&'static str] {
        use metrics::_cluster::*;

        &[health::SUBSYSTEM]
    }

    /// /_nodes subsystems
    pub fn nodes_subsystems() -> &'static [&'static str] {
        use metrics::_nodes::*;

        &[usage::SUBSYSTEM, stats::SUBSYSTEM, info::SUBSYSTEM]
    }

    /// /_stats subsystems
    pub fn stats_subsystems() -> &'static [&'static str] {
        use metrics::_stats::*;

        &[_all::SUBSYSTEM]
    }
}

fn switch_to_string(output: &mut String, field: &'static str, switches: &ExporterMetricsSwitch) {
    output.push('\n');
    output.push_str(&format!("{}:", field));
    for (k, v) in switches.iter() {
        output.push('\n');
        output.push_str(&format!(" - {}: {}", k, v));
    }
}

fn collection_labels_to_string(
    output: &mut String,
    field: &'static str,
    labels: &CollectionLabels,
) {
    output.push('\n');
    output.push_str(&format!("{}:", field));
    for (k, v) in labels.iter() {
        output.push('\n');
        output.push_str(&format!(" - {}: {}", k, v.join(",")));
    }
}

fn poll_duration_to_string(
    output: &mut String,
    field: &'static str,
    labels: &ExporterPollIntervals,
) {
    output.push('\n');
    output.push_str(&format!("{}:", field));
    for (k, v) in labels.iter() {
        output.push('\n');
        output.push_str(&format!(" - {}: {:?}", k, v));
    }
}

fn vec_to_string(output: &mut String, field: &'static str, fields: &[&'static str]) {
    output.push('\n');
    output.push_str(&format!("{}:", field));
    for field in fields.iter() {
        output.push('\n');
        output.push_str(&format!(" - {}", field));
    }
}

impl fmt::Display for ExporterOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::from("Vinted Elasticsearch exporter");

        output.push('\n');
        vec_to_string(
            &mut output,
            "Available /_cat subsystems",
            Self::cat_subsystems(),
        );
        vec_to_string(
            &mut output,
            "Available /_cluster subsystems",
            Self::cluster_subsystems(),
        );
        vec_to_string(
            &mut output,
            "Available /_nodes subsystems",
            Self::nodes_subsystems(),
        );
        vec_to_string(
            &mut output,
            "Available /_stats subsystems",
            Self::stats_subsystems(),
        );
        output.push('\n');

        output.push('\n');
        output.push_str("Exporter settings:");
        output.push('\n');
        output.push_str(&format!("elasticsearch_url: {}", self.elasticsearch_url));
        output.push('\n');
        output.push_str(&format!(
            "elasticsearch_global_timeout: {:?}",
            self.elasticsearch_global_timeout
        ));

        collection_labels_to_string(
            &mut output,
            "elasticsearch_query_fields",
            &self.elasticsearch_query_fields,
        );

        collection_labels_to_string(
            &mut output,
            "elasticsearch_query_filter_path",
            &self.elasticsearch_query_filter_path,
        );

        poll_duration_to_string(
            &mut output,
            "elasticsearch_subsystem_timeouts",
            &self.elasticsearch_subsystem_timeouts,
        );

        collection_labels_to_string(
            &mut output,
            "elasticsearch_path_parameters",
            &self.elasticsearch_path_parameters,
        );

        collection_labels_to_string(
            &mut output,
            "exporter_skip_labels",
            &self.exporter_skip_labels,
        );
        collection_labels_to_string(
            &mut output,
            "exporter_include_labels",
            &self.exporter_include_labels,
        );
        collection_labels_to_string(
            &mut output,
            "exporter_skip_metrics",
            &self.exporter_skip_metrics,
        );

        // Exporter
        output.push('\n');
        output.push_str(&format!(
            "exporter_poll_default_interval: {:?}",
            self.exporter_poll_default_interval
        ));

        poll_duration_to_string(
            &mut output,
            "exporter_poll_intervals",
            &self.exporter_poll_intervals,
        );

        output.push('\n');
        output.push_str(&format!(
            "exporter_skip_zero_metrics: {:?}",
            self.exporter_skip_zero_metrics
        ));

        switch_to_string(
            &mut output,
            "exporter_metrics_enabled",
            &self.exporter_metrics_enabled,
        );

        output.push('\n');
        output.push_str(&format!(
            "exporter_metadata_refresh_interval: {:?}",
            self.exporter_metadata_refresh_interval
        ));

        output.push('\n');
        output.push_str(&format!(
            "exporter_metrics_lifetime_default_interval: {:?}",
            self.exporter_metrics_lifetime_default_interval
        ));

        poll_duration_to_string(
            &mut output,
            "exporter_metrics_lifetime_interval",
            &self.exporter_metrics_lifetime_interval,
        );

        output.push('\n');
        write!(f, "{}", output)
    }
}
