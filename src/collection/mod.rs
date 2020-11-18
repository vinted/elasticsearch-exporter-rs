use prometheus::{default_registry, GaugeVec, HistogramOpts, HistogramVec, Opts};
use std::collections::HashMap;

use crate::{
    metric::{Metric, MetricType},
    ExporterOptions, Labels,
};

/// Generic collector of metrics
#[derive(Debug)]
pub struct Collection {
    gauges: HashMap<String, GaugeVec>,
    histogram: HashMap<String, HistogramVec>,
    subsystem: &'static str,
    /// Remove metrics from registry
    pub skip_metrics: Vec<String>,
    /// Skip unwanted labels
    pub skip_labels: Vec<String>,
    /// Include labels into metrics, this increase metric cardinality
    pub include_labels: Vec<String>,
    /// Constant metric labels
    pub const_labels: HashMap<String, String>,
    /// Exporter options
    options: ExporterOptions,
}

impl Collection {
    /// Initialize collection with given exporter options and subsystem,
    /// such as: cat_indices, cat_shards, etc.
    pub fn new(subsystem: &'static str, options: ExporterOptions) -> Self {
        Self {
            subsystem,
            options,
            skip_metrics: vec![],
            skip_labels: vec![],
            include_labels: vec![],
            const_labels: HashMap::new(),
            gauges: HashMap::new(),
            histogram: HashMap::new(),
        }
    }

    /// Insert Gauge type metric into collection
    pub fn insert_gauge(
        &mut self,
        key: &str,
        value: f64,
        labels: &Labels,
        key_postfix: Option<&'static str>,
    ) -> Result<(), prometheus::Error> {
        let keys = || labels.keys().map(|s| s.as_str()).collect::<Vec<&str>>();
        let values = labels.values().map(|s| s.as_str()).collect::<Vec<&str>>();

        if let Some(gauge) = self.gauges.get(key) {
            gauge.with_label_values(&values).set(value);
        } else {
            let mut metric_key = key.to_string();

            if let Some(postfix) = key_postfix {
                metric_key.push_str(postfix);
            }

            let new_gauge = GaugeVec::new(
                Opts::new(metric_key, key)
                    .const_labels(self.const_labels.clone())
                    .subsystem(self.subsystem)
                    .namespace(crate::NAMESPACE),
                &keys(),
            )?;

            // Register new metric
            default_registry().register(Box::new(new_gauge.clone()))?;

            new_gauge.with_label_values(&values).set(value);

            let _ = self.gauges.insert(key.to_string(), new_gauge);
        }

        Ok(())
    }

    /// Insert Histogram type metric into collection
    pub fn insert_histogram(
        &mut self,
        key: &str,
        value: f64,
        labels: &Labels,
        key_postfix: Option<&'static str>,
    ) -> Result<(), prometheus::Error> {
        let keys = || labels.keys().map(|s| s.as_str()).collect::<Vec<&str>>();
        let values = labels.values().map(|s| s.as_str()).collect::<Vec<&str>>();

        if let Some(gauge) = self.histogram.get(key) {
            gauge.with_label_values(&values).observe(value);
        } else {
            let mut metric_key = key.to_string();

            if let Some(postfix) = key_postfix {
                metric_key.push_str(postfix);
            }

            let new_histogram = HistogramVec::new(
                HistogramOpts::new(metric_key, key)
                    .const_labels(self.const_labels.clone())
                    .subsystem(self.subsystem)
                    .buckets(self.options.exporter_histogram_buckets.clone())
                    .namespace(crate::NAMESPACE),
                &keys(),
            )?;

            // Register new metric
            default_registry().register(Box::new(new_histogram.clone()))?;

            new_histogram.with_label_values(&values).observe(value);

            let _ = self.histogram.insert(key.to_string(), new_histogram);
        }

        Ok(())
    }

    /// Return metric subsystem e.g.: cat_indices, cat_nodes, etc.
    pub fn subsystem(&self) -> &'static str {
        self.subsystem
    }

    /// Collect given metrics
    pub fn collect(&mut self, mut metrics: Vec<Metric>) -> Result<(), prometheus::Error> {
        let mut labels = Labels::new();

        metrics.retain(|metric| match metric.metric_type() {
            MetricType::Label(label) => {
                if self.include_labels.contains(&metric.string_ref()) {
                    let _ = labels.insert(metric.key().to_string(), label.to_string());
                }
                false
            }
            _ => {
                !self.skip_labels.contains(&metric.string_ref())
                    && !self.skip_metrics.contains(&metric.string_ref())
            }
        });

        for metric in metrics.into_iter() {
            trace!("Collection metric: {:?}", metric);

            match metric.metric_type() {
                MetricType::Switch(value) => {
                    let _ = self.insert_gauge(&metric.key(), *value as f64, &labels, None)?;
                }
                MetricType::Bytes(value) => {
                    if self.options.exporter_skip_zero_metrics && value == &0 {
                        continue;
                    }
                    let postfix = if metric.key().ends_with("_bytes") {
                        None
                    } else {
                        Some("_bytes")
                    };
                    let _ = self.insert_gauge(&metric.key(), *value as f64, &labels, postfix)?;
                }
                MetricType::GaugeF(value) => {
                    if self.options.exporter_skip_zero_metrics && value == &0.0 {
                        continue;
                    }
                    let _ = self.insert_gauge(&metric.key(), *value, &labels, None)?;
                }
                MetricType::Gauge(value) => {
                    if self.options.exporter_skip_zero_metrics && value == &0 {
                        continue;
                    }
                    let _ = self.insert_gauge(&metric.key(), *value as f64, &labels, None)?;
                }
                MetricType::Time(duration) => {
                    if self.options.exporter_skip_zero_metrics && duration.is_zero() {
                        continue;
                    }

                    if metric.key().contains("millis") {
                        let adjusted_key = metric.key().replace("millis", "seconds");

                        let _ = self.insert_histogram(
                            &adjusted_key,
                            duration.as_secs_f64(),
                            &labels,
                            None,
                        )?;
                    } else {
                        let postfix = if metric.key().ends_with("_seconds") {
                            None
                        } else {
                            Some("_seconds")
                        };

                        let _ = self.insert_histogram(
                            &metric.key(),
                            duration.as_secs_f64(),
                            &labels,
                            postfix,
                        )?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
