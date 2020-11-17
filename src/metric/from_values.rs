use serde_json::Value;

use super::{from_value, Metrics};

/// Build vector of metrics from JSON vector values
pub fn from_values(values: Vec<Value>) -> Vec<Metrics> {
    let mut metrics: Vec<Metrics> = Vec::new();

    for value in values.into_iter() {
        metrics.extend(from_value(value));
    }

    metrics
}
