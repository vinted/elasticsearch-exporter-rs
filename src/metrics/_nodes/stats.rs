use elasticsearch::nodes::NodesStatsParts;

use super::responses::NodesResponse;

pub(crate) const SUBSYSTEM: &str = "nodes_stats";

// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-nodes-stats.html
async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let fields = exporter.options().query_fields_for_subsystem(SUBSYSTEM);
    let path_params = exporter.options().path_parameters_for_subsystem(SUBSYSTEM);

    let nodes = exporter.client().nodes();

    let mut nodes_stats = nodes
        .stats(NodesStatsParts::Metric(&path_params))
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM));

    if !fields.is_empty() {
        nodes_stats = nodes_stats.fields(&fields);
    }

    let response = nodes_stats.send().await?;

    let values = response
        .json::<NodesResponse>()
        .await?
        .into_values(exporter.nodes_metadata(), REMOVE_KEYS)
        .await;

    Ok(metric::from_values(values))
}

// NOTE:
// enabling adaptive_selection exposes metrics in nanoseconds, e.g.: "avg_response_time_ns": 196669342
const REMOVE_KEYS: &[&str] = &[
    "timestamp",
    "attributes",
    "cgroup",
    "adaptive_selection",
    "mapped - 'non-volatile memory'",
    "pipelines",
    "classes",
    "script",
];

crate::poll_metrics!();

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        metadata::node_data::{NodeData, NodeDataMap},
        metric::{Metric, MetricType},
    };
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_nodes_stats() {
        let stats: NodesResponse =
            serde_json::from_str(include_str!("../../tests/files/nodes_stats.json"))
                .expect("valid json");

        let expected_name: String = "m1-nodename.example.com".into();

        let mut metadata = NodeDataMap::new();
        let _ = metadata.insert(
            "U-WnGaTpRxucgde3miiDWw".into(),
            NodeData {
                name: expected_name.clone(),
                ..Default::default()
            },
        );
        let metadata = RwLock::new(metadata);

        let values = stats.into_values(&metadata, &["timestamp"]).await;
        assert!(!values.is_empty());
        // When keys to remove: "timestamp"
        assert!(values[0].get("timestamp").is_none());

        let value = values.last().unwrap().as_object().unwrap();
        assert_eq!(value.get("name").unwrap().as_str().unwrap(), expected_name);

        let stats: NodesResponse =
            serde_json::from_str(include_str!("../../tests/files/nodes_stats.json"))
                .expect("valid json");
        // When keys remove empty
        let values = stats.into_values(&metadata, &[]).await;
        assert!(!values.is_empty());

        let values = values[0].as_object().unwrap();
        assert_eq!(values.get("name").unwrap().as_str().unwrap(), expected_name);

        assert!(values.get("timestamp").is_some());
    }

    #[tokio::test]
    async fn test_nodes_metrics() {
        let stats: NodesResponse =
            serde_json::from_str(include_str!("../../tests/files/nodes_stats.json"))
                .expect("valid json");

        let expected_name: String = "m1-nodename.example.com".into();

        let mut metadata = NodeDataMap::new();
        let _ = metadata.insert(
            "U-WnGaTpRxucgde3miiDWw".into(),
            NodeData {
                name: expected_name.clone(),
                ..Default::default()
            },
        );
        let metadata = RwLock::new(metadata);
        let values = stats.into_values(&metadata, &["timestamp"]).await;

        let metrics = metric::from_values(values);

        let expected = vec![
            Metric(
                "fs_io_stats_devices_device_name".into(),
                MetricType::Label("sda4".into()),
            ),
            Metric("ip".into(), MetricType::Label("".into())),
            Metric("name".into(), MetricType::Label(expected_name.clone())),
            Metric(
                "fs_io_stats_devices_operations".into(),
                MetricType::Gauge(216732278),
            ),
            Metric(
                "fs_io_stats_devices_read_kilobytes".into(),
                MetricType::Bytes(21143552),
            ),
            Metric(
                "fs_io_stats_devices_read_operations".into(),
                MetricType::Gauge(1119),
            ),
            Metric("vin_cluster_version".into(), MetricType::Label("".into())),
            Metric(
                "fs_io_stats_devices_write_kilobytes".into(),
                MetricType::Bytes(2455948251136),
            ),
            Metric(
                "fs_io_stats_devices_write_operations".into(),
                MetricType::Gauge(216731159),
            ),
        ];

        assert!(
            metrics.contains(&expected),
            "got {:?}\nexpected {:?}",
            metrics,
            expected
        );
    }
}
