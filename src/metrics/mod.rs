pub(crate) mod _cat;
pub(crate) mod _cluster;
pub(crate) mod _nodes;

// TODO: add metrics of
// - https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-pending.html
// - https://www.elastic.co/guide/en/elasticsearch/reference/current/tasks.html
// - https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-state.html

/// Convenience macro to poll metrics
#[macro_export]
macro_rules! poll_metrics {
    () => {
        use crate::collection::Collection;
        use crate::metric::{self, Metrics};
        use crate::Exporter;
        use futures_util::StreamExt;
        #[allow(unused)]
        use serde_json::Value;

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

            // TODO: add random delay
            let start = tokio::time::Instant::now();

            let poll_interval = exporter
                .0
                .options
                .exporter_poll_intervals
                .get(SUBSYSTEM)
                .unwrap_or(&exporter.0.options.exporter_poll_default_interval);

            info!(
                "Starting subsystem: {} with poll interval: {:?}",
                SUBSYSTEM, poll_interval
            );

            let mut interval = tokio::time::interval_at(start, *poll_interval);

            // TODO: add metric how long it takes to scape subsystem
            while interval.next().await.is_some() {
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
            }
        }
    };
}
