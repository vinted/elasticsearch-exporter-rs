use serde_json::Value;
use std::convert::TryFrom;

mod metric_error;
mod metric_type;

pub(crate) use metric_error::MetricError;
pub use metric_type::MetricType;

pub(crate) type RawMetric<'s> = (&'s str, &'s Value);

/// Metric consisting of Key and parsed metric type
#[derive(Debug, PartialEq)]
pub struct Metric(pub(crate) String, pub(crate) MetricType);

/// Vector of metrics for convenience
pub type Metrics = Vec<Metric>;

mod from;
pub use from::{from_value, from_values};

impl Metric {
    /// Return metric key
    pub fn key(&self) -> &str {
        &self.0
    }

    /// String reference
    pub fn string_ref(&self) -> &String {
        &self.0
    }

    /// Get metric type
    pub fn metric_type(&self) -> &MetricType {
        &self.1
    }
}

impl<'s> TryFrom<RawMetric<'s>> for Metric {
    type Error = MetricError;

    fn try_from(metric: RawMetric) -> Result<Self, MetricError> {
        let mut key: String = metric.0.replace(".", "_").replace("-", "_");

        let underscore_index = key.rfind('_').unwrap_or(0);

        let shift = if key.contains('_') { 1 } else { 0 };

        let last = key
            .get(underscore_index + shift..key.len())
            .unwrap_or("UNKNOWN");

        debug_assert!(!last.contains('_'));
        debug_assert!(!last.contains('.'));
        debug_assert!(!last.contains(' '));

        let metric_type = MetricType::try_from((last, metric.1))?;

        key = key
            .replace("_kilobytes", "_bytes")
            .replace("_millis", "_seconds")
            .replace(" ", "_")
            .replace(":", "_")
            .replace("/", "_")
            .replace("\\", "_")
            .replace("[", ":")
            .replace("]", ":")
            .to_lowercase();

        debug_assert!(!key.contains(' '), "Key contains space: {}", key);

        Ok(Self(key, metric_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_raw_metric() {
        use std::time::Duration;

        let metric = "elasticsearch.test_metric.total".to_string();
        let raw: RawMetric = (&metric, &Value::from("2299291"));

        let m = Metric::try_from(raw);
        assert!(m.is_ok());
        let m = m.unwrap();
        assert_eq!(&m.key(), &"elasticsearch_test_metric_total");
        assert_eq!(m.metric_type(), &MetricType::Gauge(2299291));

        let metric = "elasticsearch.test_metric.time".to_string();
        let raw: RawMetric = (&metric, &Value::from("10"));

        let m = Metric::try_from(raw);
        assert!(m.is_ok());
        let m = m.unwrap();
        assert_eq!(&m.key(), &"elasticsearch_test_metric_time");
        assert_eq!(
            m.metric_type(),
            &MetricType::Time(Duration::from_millis(10))
        );

        let metric = "thread_pool_security-crypto_queue_size".to_string();
        let raw: RawMetric = (&metric, &Value::from("1000"));

        let m = Metric::try_from(raw);
        assert!(m.is_ok());
        let m = m.unwrap();
        assert_eq!(&m.key(), &"thread_pool_security_crypto_queue_size");

        let metric = "jvm_gc_collectors_G1 Concurrent GC_collection_count".to_string();
        let raw: RawMetric = (&metric, &Value::from("1000"));

        let m = Metric::try_from(raw);
        assert!(m.is_ok());
        let m = m.unwrap();
        assert_eq!(
            &m.key(),
            &"jvm_gc_collectors_g1_concurrent_gc_collection_count"
        );
    }

    #[test]
    fn test_try_from_raw_metric_normalize_names() {
        let metric = "transport_actions_cluster:monitor/nodes/info[n]_requests_count".to_string();
        let raw: RawMetric = (&metric, &Value::from("2"));

        let m = Metric::try_from(raw);
        assert!(m.is_ok());
        let m = m.unwrap();
        assert_eq!(
            &m.key(),
            &"transport_actions_cluster_monitor_nodes_info:n:_requests_count"
        );
        assert_eq!(m.metric_type(), &MetricType::Gauge(2));

        let metric =
            "transport_actions_internal:cluster/coordination/join/ping_requests_count".to_string();
        let raw: RawMetric = (&metric, &Value::from("2"));

        let m = Metric::try_from(raw);
        assert!(m.is_ok());
        let m = m.unwrap();
        assert_eq!(
            &m.key(),
            &"transport_actions_internal_cluster_coordination_join_ping_requests_count"
        );
        assert_eq!(m.metric_type(), &MetricType::Gauge(2));
    }
}
