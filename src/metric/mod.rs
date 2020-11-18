use serde_json::Value;
use std::convert::TryFrom;

mod metric_error;
mod metric_type;

pub(crate) use metric_error::MetricError;
pub use metric_type::MetricType;

pub(crate) type RawMetric<'s> = (&'s str, &'s Value);

/// Metric consisting of Key and parsed metric type
#[derive(Debug, PartialEq)]
pub struct Metric(String, MetricType);

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
        let key: String = metric.0.replace(".", "_");

        let underscore_index = key.rfind('_').unwrap_or(0);

        let shift = if key.contains("_") { 1 } else { 0 };

        let last = key
            .get(underscore_index + shift..key.len())
            .unwrap_or("UNKNOWN");

        debug_assert!(!last.contains("_"));
        debug_assert!(!last.contains("."));

        let metric_type = MetricType::try_from((last, metric.1))?;

        Ok(Self(key, metric_type))
    }
}

#[test]
fn test_try_from_raw_metric() {
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
}
