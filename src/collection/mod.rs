use prometheus::{default_registry, GaugeVec, IntGaugeVec, Opts};
use std::collections::HashMap;

use crate::{
    metric::{Metric, MetricType},
    ExporterOptions, Labels,
};

/// Generic collector of metrics
#[derive(Debug)]
pub struct Collection {
    gauges: HashMap<String, IntGaugeVec>,
    fgauges: HashMap<String, GaugeVec>,
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
            fgauges: HashMap::new(),
        }
    }

    /// Insert Gauge type metric into collection
    pub fn insert_fgauge(
        &mut self,
        key: &str,
        value: f64,
        labels: &Labels,
        key_postfix: Option<&'static str>,
    ) -> Result<(), prometheus::Error> {
        let keys = || labels.keys().map(|s| s.as_str()).collect::<Vec<&str>>();
        let values = labels.values().map(|s| s.as_str()).collect::<Vec<&str>>();

        if let Some(gauge) = self.fgauges.get(key) {
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

            let _ = self.fgauges.insert(key.to_string(), new_gauge);
        }

        Ok(())
    }

    /// Insert Gauge type metric into collection
    pub fn insert_gauge(
        &mut self,
        key: &str,
        value: i64,
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

            let new_gauge = IntGaugeVec::new(
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
                    let _ = self.insert_gauge(&metric.key(), *value as i64, &labels, None)?;
                }
                MetricType::Bytes(value) => {
                    if self.options.exporter_skip_zero_metrics && value == &0 {
                        continue;
                    }
                    // /_cat/recovery has key name `bytes`
                    let postfix = if metric.key().ends_with("bytes") {
                        None
                    } else {
                        Some("_bytes")
                    };
                    let _ = self.insert_gauge(&metric.key(), *value, &labels, postfix)?;
                }
                MetricType::GaugeF(value) => {
                    // is_normal: returns true if the number is neither zero, infinite, subnormal, or NaN.
                    if self.options.exporter_skip_zero_metrics && !value.is_normal() {
                        continue;
                    }
                    let _ = self.insert_fgauge(&metric.key(), *value, &labels, None)?;
                }
                MetricType::Gauge(value) => {
                    if self.options.exporter_skip_zero_metrics && value == &0 {
                        continue;
                    }
                    let _ = self.insert_gauge(&metric.key(), *value, &labels, None)?;
                }
                MetricType::Time(duration) => {
                    let secs = duration.as_secs_f64();

                    if self.options.exporter_skip_zero_metrics && !secs.is_normal() {
                        continue;
                    }

                    if metric.key().contains("millis") {
                        let adjusted_key = metric.key().replace("millis", "seconds");

                        let _ = self.insert_fgauge(&adjusted_key, secs, &labels, None)?;
                    } else {
                        let postfix = if metric.key().ends_with("_seconds") {
                            None
                        } else {
                            Some("_seconds")
                        };

                        let _ = self.insert_fgauge(&metric.key(), secs, &labels, postfix)?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[test]
fn test_float_is_zero() {
    let num: f64 = 0.000000000000000000000000000000000000000000000000000000000000000000001;
    assert!(num != 0.0);
    assert!(num.is_normal());

    let zero: f64 = 0.0;
    assert!(!zero.is_normal());

    let negative: f64 = -0.000000000000000000000000000000000000000000000000000000000000000000001;
    assert!(negative.is_normal());
}
