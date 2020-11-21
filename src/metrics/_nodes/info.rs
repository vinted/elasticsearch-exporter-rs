use elasticsearch::nodes::NodesInfoParts;

use super::responses::NodesResponse;

pub(crate) const SUBSYSTEM: &'static str = "nodes_info";

// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-nodes-stats.html
async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .nodes()
        .info(NodesInfoParts::Metric(
            &exporter.options().path_parameters_for_subsystem(SUBSYSTEM),
        ))
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .send()
        .await?;

    let values = response
        .json::<NodesResponse>()
        .await?
        .into_values(exporter.metadata(), REMOVE_KEYS)
        .await;

    Ok(metric::from_values(values))
}

const REMOVE_KEYS: &[&'static str; 15] = &[
    "aggregations",
    "timestamp",
    "plugins",
    "modules",
    "ingest",
    "input_arguments",
    "memory_pools",
    "gc_collectors",
    "build_hash",
    "build_type",
    "build_flavor",
    "using_compressed_ordinary_object_pointers",
    "vm_vendor",
    "arch",
    "settings",
];

crate::poll_metrics!();

#[tokio::test]
async fn test_nodes_info() {
    use tokio::sync::RwLock;

    use crate::metadata::node_data::{NodeData, NodeDataMap};

    let usage: NodesResponse =
        serde_json::from_str(include_str!("../../tests/files/nodes_info.json"))
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

    let values = usage.into_values(&metadata, &["timestamp"]).await;
    assert!(!values.is_empty());

    let value = values[0].as_object().unwrap();
    assert_eq!(value.get("name").unwrap().as_str().unwrap(), expected_name);
    assert!(value.get("timestamp").is_none());
}
