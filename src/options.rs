use std::fmt;
use std::time::Duration;
use url::Url;

use crate::{CollectionLabels, ExporterMetricsSwitch, ExporterPollIntervals};

/// Elasticsearch exporter options
#[derive(Debug, Clone)]
pub struct ExporterOptions {
    /// Elasticsearch cluster url
    pub elasticsearch_url: Url,
    /// Global HTTP request timeout
    pub elasticsearch_global_timeout: Duration,
    /// Elasticsearch /_nodes/stats fields comma-separated list or
    /// wildcard expressions of fields to include in the statistics.
    pub elasticsearch_nodes_stats_fields: Vec<String>,
    /// Exporter timeout for subsystems, in case subsystem timeout is not defined
    /// default global timeout is used
    pub elasticsearch_subsystem_timeouts: ExporterPollIntervals,

    /// Exporter labels to skip
    pub exporter_skip_labels: CollectionLabels,
    /// Exporter labels to include, caution this may increase metric cardinality
    pub exporter_include_labels: CollectionLabels,
    /// Exporter labels to skip completely such as segment "id"
    pub exporter_skip_metrics: CollectionLabels,
    /// Metrics polling interval
    pub exporter_poll_default_interval: Duration,
    /// Exporter skip zero metrics
    pub exporter_poll_intervals: ExporterPollIntervals,
    /// Metrics histogram buckets
    pub exporter_histogram_buckets: Vec<f64>,
    /// Exporter skip zero metrics
    pub exporter_skip_zero_metrics: bool,
    /// Exporter metrics switch either ON or OFF
    pub exporter_metrics_enabled: ExporterMetricsSwitch,
    /// Exporter metadata refresh interval
    pub exporter_metadata_refresh_interval: Duration,
}

impl ExporterOptions {
    /// Check if metric is enabled
    pub fn is_metric_enabled(&self, subsystem: &'static str) -> bool {
        self.exporter_metrics_enabled.contains_key(subsystem)
    }

    /// Get timeout for subsystem or fallback to global
    pub fn timeout_for_subsystem(&self, subsystem: &'static str) -> Duration {
        self.elasticsearch_subsystem_timeouts
            .get(subsystem)
            .unwrap_or(&self.elasticsearch_global_timeout)
            .clone()
    }
}

fn switch_to_string(output: &mut String, field: &'static str, switches: &ExporterMetricsSwitch) {
    output.push_str("\n");
    output.push_str(&format!("{}:", field));
    for (k, v) in switches.iter() {
        output.push_str("\n");
        output.push_str(&format!(" - {}: {}", k, v));
    }
}

fn collection_labels_to_string(
    output: &mut String,
    field: &'static str,
    labels: &CollectionLabels,
) {
    output.push_str("\n");
    output.push_str(&format!("{}:", field));
    for (k, v) in labels.iter() {
        output.push_str("\n");
        output.push_str(&format!(" - {}: {}", k, v.join(",")));
    }
}

fn poll_duration_to_string(
    output: &mut String,
    field: &'static str,
    labels: &ExporterPollIntervals,
) {
    output.push_str("\n");
    output.push_str(&format!("{}:", field));
    for (k, v) in labels.iter() {
        output.push_str("\n");
        output.push_str(&format!(" - {}: {:?}", k, v));
    }
}

impl fmt::Display for ExporterOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::from("Vinted Elasticsearch exporter");

        output.push_str("\n");
        output.push_str(&format!("elasticsearch_url: {}", self.elasticsearch_url));
        output.push_str("\n");
        output.push_str(&format!(
            "elasticsearch_global_timeout: {:?}",
            self.elasticsearch_global_timeout
        ));
        output.push_str("\n");
        output.push_str(&format!(
            "elasticsearch_nodes_stats_fields: {}",
            self.elasticsearch_nodes_stats_fields.join(",")
        ));

        poll_duration_to_string(
            &mut output,
            "elasticsearch_subsystem_timeouts",
            &self.elasticsearch_subsystem_timeouts,
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
        output.push_str("\n");
        output.push_str(&format!(
            "exporter_poll_default_interval: {:?}",
            self.exporter_poll_default_interval
        ));

        poll_duration_to_string(
            &mut output,
            "exporter_poll_intervals",
            &self.exporter_poll_intervals,
        );

        output.push_str("\n");
        output.push_str(&format!(
            "exporter_histogram_buckets: {:?} in seconds",
            self.exporter_histogram_buckets
        ));

        output.push_str("\n");
        output.push_str(&format!(
            "exporter_skip_zero_metrics: {:?}",
            self.exporter_skip_zero_metrics
        ));

        switch_to_string(
            &mut output,
            "exporter_metrics_enabled",
            &self.exporter_metrics_enabled,
        );

        output.push_str("\n");
        output.push_str(&format!(
            "exporter_metadata_refresh_interval: {:?}",
            self.exporter_metadata_refresh_interval
        ));

        output.push_str("\n");
        write!(f, "{}", output)
    }
}
