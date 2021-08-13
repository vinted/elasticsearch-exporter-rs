use prometheus::{default_registry, GaugeVec, IntGaugeVec, Opts};
use std::collections::HashMap;

/// Lifetime of a metric based on heartbeat
pub mod lifetime;

use crate::{
    metric::{Metric, MetricType},
    ExporterOptions, Labels,
};

/// Generic collector of metrics
#[derive(Debug)]
pub struct Collection {
    /// Integer gauges of collection
    pub gauges: HashMap<String, IntGaugeVec>,
    /// Float gauges of collection
    pub fgauges: HashMap<String, GaugeVec>,
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
    /// Metric lifetime is used to remove stale metrics
    pub gauges_lifetime: lifetime::MetricLifetimeMap,
    /// Metric lifetime is used to remove stale metrics
    pub fgauges_lifetime: lifetime::MetricLifetimeMap,
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
            gauges_lifetime: Default::default(),
            fgauges_lifetime: Default::default(),
        }
    }

    /// Insert Gauge type metric into collection
    pub fn insert_fgauge(
        &mut self,
        key: &str,
        value: f64,
        labels: &Labels,
        key_postfix: Option<&'static str>,
        skippable: bool,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), prometheus::Error> {
        let set_labels = |gauge: &GaugeVec,
                          lifetime: &mut lifetime::MetricLifetimeMap|
         -> Result<(), prometheus::Error> {
            // BTreeMap ensures that values returned are always sorted
            let label_values = &labels.values().map(|s| s.as_str()).collect::<Vec<&str>>();

            gauge.get_metric_with_label_values(label_values)?.set(value);

            if !label_values.is_empty() {
                let _ = lifetime
                    .entry(lifetime::hash_label(key, label_values))
                    .or_insert_with(|| {
                        lifetime::MetricLifetime::new(
                            key.to_string(),
                            labels.values().cloned().collect(),
                        )
                    })
                    .reset_heartbeat(Some(now));
            }

            Ok(())
        };

        if let Some(fgauge) = self.fgauges.get(key) {
            let _ = set_labels(fgauge, &mut self.fgauges_lifetime)?;
        } else {
            // If metric is skippable and haven't been registered skip it
            // until value is not zero
            // is_normal: returns true if the number is neither zero, infinite, subnormal, or NaN.
            if skippable && self.options.exporter_skip_zero_metrics && !value.is_normal() {
                return Ok(());
            }

            let mut metric_key = key.to_string();

            if let Some(postfix) = key_postfix {
                metric_key.push_str(postfix);
            }

            let new_fgauge = GaugeVec::new(
                Opts::new(metric_key, key)
                    .const_labels(self.const_labels.clone())
                    .subsystem(self.subsystem)
                    .namespace(crate::NAMESPACE),
                &labels.keys().map(|s| s.as_str()).collect::<Vec<&str>>(),
            )?;

            let _ = set_labels(&new_fgauge, &mut self.fgauges_lifetime)?;

            // Register new metric
            default_registry().register(Box::new(new_fgauge.clone()))?;

            let _ = self.fgauges.insert(key.to_string(), new_fgauge);
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
        skippable: bool,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), prometheus::Error> {
        let set_labels = |gauge: &IntGaugeVec,
                          lifetime: &mut lifetime::MetricLifetimeMap|
         -> Result<(), prometheus::Error> {
            // BTreeMap ensures that values returned are always sorted
            let label_values = &labels.values().map(|s| s.as_str()).collect::<Vec<&str>>();

            gauge.get_metric_with_label_values(label_values)?.set(value);

            if !label_values.is_empty() {
                let _ = lifetime
                    .entry(lifetime::hash_label(key, label_values))
                    .or_insert_with(|| {
                        lifetime::MetricLifetime::new(
                            key.to_string(),
                            labels.values().cloned().collect(),
                        )
                    })
                    .reset_heartbeat(Some(now));
            }

            Ok(())
        };

        if let Some(gauge) = self.gauges.get(key) {
            let _ = set_labels(gauge, &mut self.gauges_lifetime)?;
        } else {
            // If metric is skippable and haven't been registered skip it
            // until value is not zero
            if skippable && self.options.exporter_skip_zero_metrics && value == 0 {
                return Ok(());
            }

            let mut metric_key = key.to_string();

            if let Some(postfix) = key_postfix {
                metric_key.push_str(postfix);
            }

            let new_gauge = IntGaugeVec::new(
                Opts::new(metric_key, key)
                    .const_labels(self.const_labels.clone())
                    .subsystem(self.subsystem)
                    .namespace(crate::NAMESPACE),
                &labels.keys().map(|s| s.as_str()).collect::<Vec<&str>>(),
            )?;

            let _ = set_labels(&new_gauge, &mut self.gauges_lifetime)?;

            // Register new metric
            default_registry().register(Box::new(new_gauge.clone()))?;

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
                if self.include_labels.contains(metric.string_ref()) {
                    let _ = labels.insert(metric.key().to_string(), label.to_string());
                }
                false
            }
            _ => {
                !self.skip_labels.contains(metric.string_ref())
                    && !self.skip_metrics.contains(metric.string_ref())
            }
        });

        // Finding current time on each metric insert is expensive (requires syscall)
        // cloning is way cheaper thus initializing current time once for all collected
        // metrics batch is more correct approach
        let now = lifetime::now();

        for metric in metrics.into_iter() {
            trace!("Collection metric: {:?}", metric);

            match metric.metric_type() {
                MetricType::Switch(value) => {
                    if let Err(e) =
                        self.insert_gauge(metric.key(), *value as i64, &labels, None, false, now)
                    {
                        error!("SWITCH insert_gauge {:?} err {}", metric, e);
                        return Err(e);
                    }
                }
                MetricType::Bytes(value) => {
                    // /_cat/recovery has key name `bytes`
                    let postfix = if metric.key().ends_with("bytes") {
                        None
                    } else {
                        Some("_bytes")
                    };
                    if let Err(e) =
                        self.insert_gauge(metric.key(), *value, &labels, postfix, true, now)
                    {
                        error!("BYTES insert_gauge {:?} err {}", metric, e);
                        return Err(e);
                    }
                }
                MetricType::GaugeF(value) => {
                    if let Err(e) =
                        self.insert_fgauge(metric.key(), *value, &labels, None, true, now)
                    {
                        error!("GAUGEF insert_fgauge {:?} err {}", metric, e);
                        return Err(e);
                    }
                }
                MetricType::Gauge(value) => {
                    if let Err(e) =
                        self.insert_gauge(metric.key(), *value, &labels, None, true, now)
                    {
                        error!("GAUGE insert_gauge {:?} err {}", metric, e);
                        return Err(e);
                    }
                }
                MetricType::Time(duration) => {
                    let adjusted_key = metric.key().replace("_millis", "_seconds");

                    let postfix = if adjusted_key.ends_with("_seconds") {
                        None
                    } else {
                        Some("_seconds")
                    };

                    if let Err(e) = self.insert_fgauge(
                        &adjusted_key,
                        duration.as_secs_f64(),
                        &labels,
                        postfix,
                        true,
                        now,
                    ) {
                        error!("TIME insert_fgauge {:?} err {}", metric, e);
                        return Err(e);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_float_is_zero() {
        let num: f64 = 0.000000000000000000000000000000000000000000000000000000000000000000001;
        assert!(num != 0.0);
        assert!(num.is_normal());

        let zero: f64 = 0.0;
        assert!(!zero.is_normal());

        let negative: f64 =
            -0.000000000000000000000000000000000000000000000000000000000000000000001;
        assert!(negative.is_normal());
    }
}
