use serde_json::Value;
use std::convert::TryFrom;

use super::{Metric, MetricError, Metrics};

/// Map from key and JSON value
type SerdeMap = serde_json::Map<String, Value>;

/// Build vector of metrics from JSON vector values
pub fn from_values(values: Vec<Value>) -> Vec<Metrics> {
    let mut metrics: Vec<Metrics> = Vec::new();

    for value in values.into_iter() {
        metrics.extend(from_value(value));
    }

    metrics
}

/// Build metric from JSON value
pub fn from_value(value: Value) -> Vec<Metrics> {
    let mut output: Vec<Metrics> = Vec::new();

    // Instead of returning error print error and return
    // any metrics that were processed
    match _from_value("".into(), &mut output, &value) {
        Ok(metrics) => {
            debug_assert!(metrics.is_empty());
        }
        Err(e) => {
            error!("from_value err {}", e);
        }
    }

    output
}

fn _from_value<'f>(
    prefix: &str,
    output: &mut Vec<Metrics>,
    value: &'f Value,
) -> Result<Metrics, MetricError> {
    let mut metrics = Metrics::new();

    if value.is_number()
        || value.is_boolean()
        || value.is_string()
        || value.is_number()
        || value.is_null()
    {
        metrics.push(Metric::try_from((prefix, value))?);
    } else if let Some(obj) = value.as_object() {
        let _ = _from_map(prefix, output, obj)?;
    } else if let Some(array) = value.as_array() {
        let _ = from_array(prefix, output, array)?;
    } else {
        unreachable!()
    }
    Ok(metrics)
}

fn _from_map(prefix: &str, output: &mut Vec<Metrics>, map: &SerdeMap) -> Result<(), MetricError> {
    let mut metrics = Metrics::new();

    for (key, value) in map.iter() {
        let inner_metrics = if prefix == "" {
            _from_value(key, output, value)?
        } else {
            _from_value(&format!("{}_{}", prefix, key), output, value)?
        };

        if !inner_metrics.is_empty() {
            metrics.extend(inner_metrics);
        }
    }

    if !metrics.is_empty() {
        output.push(metrics);
    }

    Ok(())
}

fn from_array<'f>(
    prefix: &str,
    output: &mut Vec<Metrics>,
    values: &'f Vec<Value>,
) -> Result<(), MetricError> {
    let mut metrics = Metrics::new();

    for value in values.iter() {
        metrics.extend(_from_value(prefix, output, value)?);
    }

    if !metrics.is_empty() {
        output.push(metrics);
    }

    Ok(())
}

#[test]
fn test_cluster_stats_from_map() {
    use super::MetricType;

    let value: Value =
        serde_json::from_str(include_str!("../tests/files/types.json")).expect("valid json");

    let metrics = from_value(value);

    let expected = vec![
        Metric("_nodes_failed".into(), MetricType::Gauge(9329292)),
        Metric("_nodes_some_float".into(), MetricType::GaugeF(1.13)),
        Metric("_nodes_some_total".into(), MetricType::Gauge(22)),
    ];
    assert!(metrics.contains(&expected));

    let expected = vec![Metric("array_map".into(), MetricType::Gauge(1))];
    assert!(metrics.contains(&expected));

    let expected = vec![
        Metric("array_second_dimension".into(), MetricType::Gauge(14)),
        Metric(
            "array_second_my_label".into(),
            MetricType::Label("super".into()),
        ),
    ];
    assert!(metrics.contains(&expected));

    let expected = vec![
        Metric("top_level_bytes".into(), MetricType::Bytes(2)),
        Metric("top_level_one".into(), MetricType::Gauge(1)),
        Metric("top_level_size".into(), MetricType::Bytes(3)),
    ];
    assert!(metrics.contains(&expected));
}
