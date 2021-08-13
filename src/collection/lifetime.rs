use chrono::{DateTime, Utc};
use fnv::FnvHasher;
use std::collections::HashMap;
use std::hash::Hasher;

pub(crate) fn hash_label(key: &str, values: &[&str]) -> u64 {
    let mut h = FnvHasher::default();
    h.write(key.as_bytes());

    for val in values.iter() {
        h.write(val.as_bytes());
    }

    h.finish()
}

/// Return current date time
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// Metric Lifetime map
pub type MetricLifetimeMap = HashMap<u64, MetricLifetime>;

/// Metric lifetime
#[derive(Debug)]
pub struct MetricLifetime {
    last_hearbeat: DateTime<Utc>,
    /// Metric key
    pub metric_key: String,
    /// Label values
    pub label_values: Vec<String>,
}

impl MetricLifetime {
    /// Initialize MetricLifetime
    pub fn new(metric_key: String, label_values: Vec<String>) -> Self {
        Self {
            last_hearbeat: now(),
            metric_key,
            label_values,
        }
    }

    /// Set heartbeat
    pub fn reset_heartbeat(&mut self, last: Option<DateTime<Utc>>) -> &mut Self {
        self.last_hearbeat = last.unwrap_or_else(now);
        self
    }

    /// Check if metric is outdated based on last metric heartbeat
    pub fn is_outdated(&self, date: DateTime<Utc>) -> bool {
        self.last_hearbeat < date
    }
}
