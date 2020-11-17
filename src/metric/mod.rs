use serde_json::Value;
use std::borrow::ToOwned;
use std::convert::TryFrom;
use std::error::Error as StdError;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::time::Duration;

type RawMetric<'s> = (&'s str, &'s Value);

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

    /// Null value
    Null,
}

impl<'s> TryFrom<RawMetric<'s>> for MetricType {
    type Error = MetricError;

    fn try_from(metric: RawMetric) -> Result<Self, MetricError> {
        let value: &Value = metric.1;

        let unknown = || MetricError::unknown(metric.0.to_owned(), Some(value.clone()));

        let parse_i64 = || -> Result<i64, MetricError> {
            if value.is_number() {
                Ok(value.as_i64().unwrap_or(0))
            } else {
                value
                    .as_str()
                    .map(|n| n.parse::<i64>())
                    .ok_or(unknown())?
                    .map_err(|e| MetricError::from_parse_int(e, Some(value.clone())))
            }
        };

        let parse_f64 = || -> Result<f64, MetricError> {
            if value.is_f64() {
                Ok(value.as_f64().unwrap_or(0.0))
            } else {
                value
                    .as_str()
                    .map(|n| n.replace("%", "").parse::<f64>())
                    .ok_or(unknown())?
                    .map_err(|e| MetricError::from_parse_float(e, Some(value.clone())))
            }
        };

        if value.is_boolean() {
            return Ok(MetricType::Switch(if value.as_bool().unwrap_or(false) {
                1
            } else {
                0
            }));
        }

        if value.is_null() {
            return Ok(MetricType::Null);
        }

        match metric.0 {
            "size" | "memory" | "store" | "bytes" => return Ok(MetricType::Bytes(parse_i64()?)),
            "epoch" | "timestamp" | "date" | "time" | "millis" | "alive" => {
                return Ok(MetricType::Time(Duration::from_millis(
                    parse_i64().unwrap_or(0) as u64,
                )))
            }
            _ => {
                if value.is_number() {
                    if value.is_i64() {
                        return Ok(MetricType::Gauge(parse_i64()?));
                    } else {
                        return Ok(MetricType::GaugeF(parse_f64()?));
                    }
                }
            }
        }

        // TODO: rethink list matching, label could be matched by default with
        // attempt to number before return
        match metric.0 {
            // timed_out
            "out" | "value" | "committed" | "searchable" | "compound" | "throttled" => {
                Ok(MetricType::Switch(if value.as_bool().unwrap_or(false) {
                    1
                } else {
                    0
                }))
            }

            // Special cases
            // _cat/health: elasticsearch_cat_health_node_data{cluster="testing"}
            // _cat/shards: "path.data": "/var/lib/elasticsearch/m1/nodes/0"
            "data" => match parse_i64() {
                Ok(number) => Ok(MetricType::Gauge(number)),
                Err(_) => Ok(MetricType::Label(
                    value.as_str().ok_or(unknown())?.to_owned(),
                )),
            },

            "primaries" | "min" | "max" | "successful" | "nodes" | "fetch" | "order"
            | "largest" | "rejected" | "completed" | "queue" | "active" | "core" | "tasks"
            | "relo" | "unassign" | "init" | "files" | "ops" | "recovered" | "generation"
            | "contexts" | "listeners" | "pri" | "rep" | "docs" | "count" | "pid"
            | "compilations" | "deleted" | "shards" | "indices" | "checkpoint" | "avail"
            | "used" | "cpu" | "triggered" | "evictions" | "failed" | "total" | "current" => {
                Ok(MetricType::Gauge(parse_i64()?))
            }

            "avg" | "1m" | "5m" | "15m" | "number" | "percent" => {
                Ok(MetricType::GaugeF(parse_f64()?))
            }

            "cluster" | "repository" | "snapshot" | "stage" | "uuid" | "component" | "master"
            | "role" | "uptime" | "alias" | "filter" | "search" | "flavor" | "string"
            | "address" | "health" | "build" | "node" | "state" | "patterns" | "of" | "segment"
            | "host" | "ip" | "prirep" | "id" | "status" | "at" | "for" | "details" | "reason"
            | "port" | "attr" | "field" | "shard" | "index" | "name" | "type" | "version"
            | "jdk" | "description" => Ok(MetricType::Label(
                value.as_str().ok_or(unknown())?.to_owned(),
            )),
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
pub struct MetricError {
    kind: Kind,
    value: Option<Value>,
}

impl MetricError {
    fn unknown(s: String, value: Option<Value>) -> Self {
        MetricError {
            kind: Kind::Unknown(s),
            value,
        }
    }

    fn from_parse_float(e: ParseFloatError, value: Option<Value>) -> Self {
        MetricError {
            kind: Kind::ParseFloat(e),
            value,
        }
    }

    fn from_parse_int(e: ParseIntError, value: Option<Value>) -> Self {
        MetricError {
            kind: Kind::ParseInt(e),
            value,
        }
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
        write!(
            f,
            "MetricError kind {:?} value: {:?}",
            self.kind, self.value
        )
    }
}
