use serde_json::Value;
use std::borrow::ToOwned;
use std::convert::TryFrom;
use std::error::Error as StdError;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::time::Duration;

type RawMetric<'s> = (&'s str, &'s Value);

/// Metric consisting of Key and parsed metric type
#[derive(Debug)]
pub struct Metric(String, MetricType);

/// Vector of metrics for convenience
pub type Metrics = Vec<Metric>;

/// Build metric from JSON value
pub fn from_value(value: Value) -> Vec<Metrics> {
    let mut metrics: Vec<Metrics> = Vec::new();

    if let Some(object) = value.as_object() {
        metrics.push(
            object
                .into_iter()
                .map(|(k, v)| Metric::try_from((k.as_str(), v)))
                .filter_map(Result::ok)
                .collect::<Vec<Metric>>(),
        );
    } else {
        warn!("from_values unsupported value {}", value);
    }
    metrics
}

/// Build vector of metrics from JSON vector values
pub fn from_values(values: Vec<Value>) -> Vec<Metrics> {
    let mut metrics: Vec<Metrics> = Vec::new();

    for value in values.into_iter() {
        metrics.extend(from_value(value));
    }

    metrics
}

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

/// Parsed metric types
#[derive(Debug, PartialEq)]
pub enum MetricType {
    /// Time is parsed as duration in milliseconds
    /// duration is casted to float seconds
    Time(Duration), // Milliseconds
    /// Bytes
    Bytes(i64),
    /// Integer gauges
    Gauge(i64),
    /// Float gauges
    GaugeF(f64),
    /// Switch metrics having value of true/false
    Switch(u8),

    /// Labels e.g.: index, node, ip, etc.
    Label(String), // Everything not number
}

impl<'s> TryFrom<RawMetric<'s>> for MetricType {
    type Error = MetricError;

    fn try_from(metric: RawMetric) -> Result<Self, MetricError> {
        let value: &Value = metric.1;

        let unknown = || MetricError::unknown(metric.0.to_owned());

        let parse_i64 = || -> Result<i64, MetricError> {
            if value.is_number() {
                Ok(value.as_i64().unwrap_or(0))
            } else {
                value
                    .as_str()
                    .map(|n| n.parse::<i64>())
                    .ok_or(unknown())?
                    .map_err(MetricError::from)
            }
        };

        let parse_f64 = || -> Result<f64, MetricError> {
            if value.is_f64() {
                Ok(value.as_f64().unwrap_or(0.0))
            } else {
                value
                    .as_str()
                    .map(|n| n.parse::<f64>())
                    .ok_or(unknown())?
                    .map_err(MetricError::from)
            }
        };

        match metric.0 {
            "size" | "memory" | "store" | "bytes" => Ok(MetricType::Bytes(parse_i64()?)),
            "date" | "time" | "millis" | "alive" => Ok(MetricType::Time(Duration::from_millis(
                parse_i64().unwrap_or(0) as u64,
            ))),
            "epoch" => Ok(MetricType::Time(Duration::from_secs(parse_i64()? as u64))),

            // timed_out
            "out" | "value" => Ok(MetricType::Switch(if value.as_bool().unwrap_or(false) {
                1
            } else {
                0
            })),

            "nodes" | "fetch" | "order" | "largest" | "rejected" | "completed" | "queue"
            | "active" | "core" | "data" | "tasks" | "relo" | "unassign" | "init" | "files"
            | "ops" | "recovered" | "generation" | "max" | "contexts" | "listeners" | "pri"
            | "rep" | "docs" | "count" | "pid" | "compilations" | "deleted" | "shards"
            | "indices" | "checkpoint" | "avail" | "used" | "cpu" | "triggered" | "evictions"
            | "failed" | "total" | "current" => Ok(MetricType::Gauge(parse_i64()?)),

            "1m" | "5m" | "15m" | "number" | "percent" => Ok(MetricType::GaugeF(parse_f64()?)),

            "status" | "at" | "for" | "details" | "reason" | "sync_id" | "port" | "attr"
            | "field" | "shard" | "index" | "name" | "type" | "version" | "jdk" | "description" => {
                Ok(MetricType::Label(
                    value.as_str().ok_or(unknown())?.to_owned(),
                ))
            }

            _ => {
                if cfg!(debug_assertions) {
                    println!("Catchall metric: {:?}", metric);

                    let parsed = parse_i64().unwrap_or(-1).to_string();

                    if &parsed != "-1" && parsed.len() == value.as_str().unwrap_or("").len() {
                        println!("Unhandled metic value {:?}", metric);
                    }
                }

                Ok(MetricType::Label(
                    value.as_str().ok_or(unknown())?.to_owned(),
                ))
            }
        }
    }
}

/// Metric error wrapper of parsing errors
#[derive(Debug)]
pub struct MetricError(Kind);

impl MetricError {
    fn unknown(s: String) -> Self {
        MetricError(Kind::Unknown(s))
    }
}
impl From<ParseFloatError> for MetricError {
    fn from(e: ParseFloatError) -> Self {
        Self(Kind::ParseFloat(e))
    }
}

impl From<ParseIntError> for MetricError {
    fn from(e: ParseIntError) -> Self {
        Self(Kind::ParseInt(e))
    }
}

#[derive(Debug)]
enum Kind {
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    Unknown(String),
}

impl StdError for MetricError {}

impl fmt::Display for MetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MetricError error: {:?}", self.0)
    }
}
