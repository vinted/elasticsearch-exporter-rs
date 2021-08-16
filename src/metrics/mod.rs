pub(crate) mod _cat;
pub(crate) mod _cluster;
pub(crate) mod _nodes;
pub(crate) mod _stats;

// TODO: add metrics of
// - https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-pending.html
// - https://www.elastic.co/guide/en/elasticsearch/reference/current/tasks.html
// - https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-state.html

/// Convenience macro to poll metrics
#[macro_export]
macro_rules! poll_metrics {
    () => {
        #[allow(unused)]
        use serde_json::Value;
        use std::time::Duration;

        use crate::collection::{lifetime::MetricLifetimeMap, Collection};
        use crate::exporter_metrics::SUBSYSTEM_REQ_HISTOGRAM;
        use crate::metric::{self, Metrics};
        use crate::Exporter;

        #[allow(unused)]
        pub(crate) async fn poll(exporter: Exporter) {
            let options = exporter.options();

            let mut collection = Collection::new(SUBSYSTEM, options.clone());
            // Common to all /_cat metrics
            collection.const_labels = exporter.const_labels();

            if let Some(skip_labels) = options.exporter_skip_labels.get(SUBSYSTEM) {
                collection.skip_labels = skip_labels.clone();
            }

            if let Some(skip_metrics) = options.exporter_skip_metrics.get(SUBSYSTEM) {
                collection.skip_metrics = skip_metrics.clone();
            }

            if let Some(include_labels) = options.exporter_include_labels.get(SUBSYSTEM) {
                collection.include_labels = include_labels.clone();
            }

            let start =
                tokio::time::Instant::now() + Duration::from_millis(Exporter::random_delay());

            let poll_interval = options
                .exporter_poll_intervals
                .get(SUBSYSTEM)
                .unwrap_or(&options.exporter_poll_default_interval);

            let metric_lifetime = options
                .exporter_metrics_lifetime_interval
                .get(SUBSYSTEM)
                .unwrap_or(&options.exporter_metrics_lifetime_default_interval);

            info!(
                "Starting subsystem: {} with poll interval: {}sec lifetime: {}sec",
                SUBSYSTEM,
                poll_interval.as_secs(),
                metric_lifetime.as_secs(),
            );

            let mut interval = tokio::time::interval_at(start, *poll_interval);
            // Convert to chrono Duration by overriding variable
            let metric_lifetime = chrono::Duration::seconds(metric_lifetime.as_secs() as i64);

            loop {
                let now = chrono::Utc::now() - metric_lifetime;

                let _ = interval.tick().await;

                let timer = SUBSYSTEM_REQ_HISTOGRAM
                    .with_label_values(&[&format!("/{}", SUBSYSTEM), exporter.cluster_name()])
                    .start_timer();

                match metrics(&exporter).await {
                    Ok(metrics) => {
                        for metric in metrics.into_iter() {
                            let _ = collection.collect(metric);
                        }
                    }
                    Err(e) => {
                        error!("poll {} metrics err {}", collection.subsystem(), e);
                    }
                }

                timer.observe_duration();

                for (_, v) in collection
                    .gauges_lifetime
                    .drain_filter(|_k, v| v.is_outdated(now))
                    .collect::<MetricLifetimeMap>()
                    .iter()
                {
                    debug!(
                        "REMOVING `{}` stale metric: {} labels: {:?}",
                        SUBSYSTEM, v.metric_key, v.label_values
                    );
                    if let Some(gauge) = collection.gauges.get(&v.metric_key) {
                        gauge.remove_label_values(
                            &v.label_values
                                .iter()
                                .map(|lv| lv.as_str())
                                .collect::<Vec<&str>>(),
                        );
                    }
                }

                for (_, v) in collection
                    .fgauges_lifetime
                    .drain_filter(|_k, v| v.is_outdated(now))
                    .collect::<MetricLifetimeMap>()
                    .iter()
                {
                    debug!(
                        "REMOVING `{}` stale metric: {} labels: {:?}",
                        SUBSYSTEM, v.metric_key, v.label_values
                    );
                    if let Some(fgauge) = collection.fgauges.get(&v.metric_key) {
                        fgauge.remove_label_values(
                            &v.label_values
                                .iter()
                                .map(|lv| lv.as_str())
                                .collect::<Vec<&str>>(),
                        );
                    }
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_chrono_lifetime_difference() {
        let now = chrono::Utc::now();

        let past = now - chrono::Duration::days(3);

        assert_ne!(past, now);
        assert!(past < now);
    }
}
