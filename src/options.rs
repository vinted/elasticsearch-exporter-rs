use std::fmt;
use std::time::Duration;
use url::Url;

use crate::{CollectionLabels, ExporterMetricsSwitch, ExporterPollIntervals, Labels};

/// Elasticsearch exporter options
#[derive(Debug, Clone)]
pub struct ExporterOptions {
    /// Elasticsearch cluster url
    pub elasticsearch_url: Url,
    /// Global HTTP request timeout
    pub elasticsearch_global_timeout: Duration,
    /// Elasticsearch labels to skip
    pub elasticsearch_skip_labels: CollectionLabels,
    /// Elasticsearch labels to include, caution this may increase metric cardinality
    pub elasticsearch_include_labels: CollectionLabels,
    /// Elasticsearch labels to skip completely such as segment "id"
    pub elasticsearch_skip_metrics: CollectionLabels,
    /// Elasticsearch cat API header fields
    pub elasticsearch_cat_headers: Labels,

    /// Metrics polling interval
    pub exporter_poll_default_interval: Duration,
    /// Exporter skip zero metrics
    pub exporter_poll_intervals: ExporterPollIntervals,
    /// Metrics histogram buckets
    pub exporter_histogram_buckets: Vec<f64>,
    /// Exporter skip zero metrics
    pub exporter_skip_zero_metrics: bool,
    /// Exporter metrics switch either ON or OFF
    pub exporter_metrics_switch: ExporterMetricsSwitch,
}

impl ExporterOptions {
    /// Check if metric is enabled
    pub fn is_metric_enabled(&self, subsystem: &'static str) -> bool {
        self.exporter_metrics_switch.contains_key(subsystem)
    }
}

fn switch_to_string(output: &mut String, field: &'static str, switches: &ExporterMetricsSwitch) {
    output.push_str("\n");
    output.push_str(field);
    for (k, v) in switches.iter() {
        output.push_str("\n");
        output.push_str(&format!(" - {}: {}", k, v));
    }
}

fn labels_to_string(output: &mut String, field: &'static str, labels: &Labels) {
    output.push_str("\n");
    output.push_str(field);
    for (k, v) in labels.iter() {
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
    output.push_str(field);
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
    output.push_str(field);
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
        collection_labels_to_string(
            &mut output,
            "elasticsearch_skip_labels",
            &self.elasticsearch_skip_labels,
        );
        collection_labels_to_string(
            &mut output,
            "elasticsearch_include_labels",
            &self.elasticsearch_include_labels,
        );
        collection_labels_to_string(
            &mut output,
            "elasticsearch_skip_metrics",
            &self.elasticsearch_skip_metrics,
        );
        labels_to_string(
            &mut output,
            "elasticsearch_cat_headers",
            &self.elasticsearch_cat_headers,
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
            "exporter_metrics_switch",
            &self.exporter_metrics_switch,
        );

        output.push_str("\n");
        write!(f, "{}", output)
    }
}
