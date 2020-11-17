use std::fmt;
use std::time::Duration;
use url::Url;

use crate::{CollectionLabels, Labels};

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
    pub exporter_poll_interval: Duration,
    /// Metrics histogram buckets
    pub exporter_histogram_buckets: Vec<f64>,
    /// Exporter skip zero metrics
    pub exporter_skip_zero_metrics: bool,
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
            "exporter_poll_interval: {:?}",
            self.exporter_poll_interval
        ));

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

        write!(f, "{}", output)
    }
}
