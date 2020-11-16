pub(crate) mod _cat;

/// Convenience macro to poll metrics
#[macro_export]
macro_rules! poll_metrics {
    () => {
        use crate::collection::Collection;
        use crate::metric::{self, Metrics};
        use crate::Exporter;
        use futures_util::StreamExt;
        use serde_json::Value;

        #[allow(unused)]
        pub(crate) async fn poll(exporter: Exporter) {
            let start = tokio::time::Instant::now();
            let mut interval =
                tokio::time::interval_at(start, exporter.options.exporter_poll_interval);

            let mut collection = Collection::new(SUBSYSTEM, exporter.options.clone());
            // Common to all /_cat metrics
            collection.const_labels = exporter.const_labels.clone();

            if let Some(skip_labels) = exporter.options.elasticsearch_skip_labels.get(SUBSYSTEM) {
                collection.skip_labels = skip_labels.clone();
            }

            if let Some(skip_metrics) = exporter.options.elasticsearch_skip_metrics.get(SUBSYSTEM) {
                collection.skip_metrics = skip_metrics.clone();
            }

            if let Some(include_labels) =
                exporter.options.elasticsearch_include_labels.get(SUBSYSTEM)
            {
                collection.include_labels = include_labels.clone();
            }

            while interval.next().await.is_some() {
                match metrics(&exporter).await {
                    Ok(metrics) => {
                        for metric in metrics.into_iter() {
                            let _ = collection.collect(metric);
                        }
                    }
                    Err(e) => {
                        error!("{} metrics err {}", collection.subsystem(), e);
                    }
                }
            }
        }
    };
}
