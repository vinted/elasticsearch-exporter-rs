use elasticsearch::nodes::NodesUsageParts;

use super::responses::NodesResponse;

pub(crate) const SUBSYSTEM: &str = "nodes_usage";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .nodes()
        .usage(NodesUsageParts::None)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .send()
        .await?;

    let values = response
        .json::<NodesResponse>()
        .await?
        .into_values(exporter.nodes_metadata(), REMOVE_KEYS)
        .await;

    Ok(metric::from_values(values))
}

const REMOVE_KEYS: &[&str] = &["since", "timestamp"];

crate::poll_metrics!();

#[tokio::test]
async fn test_nodes_usage() {
    use tokio::sync::RwLock;

    use crate::metadata::node_data::{NodeData, NodeDataMap};

    let usage: NodesResponse =
        serde_json::from_str(include_str!("../../tests/files/nodes_usage.json"))
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

    let aggregations = values[0].as_object().unwrap();
    assert_eq!(
        aggregations.get("name").unwrap().as_str().unwrap(),
        expected_name
    );
    assert!(aggregations.get("timestamp").is_none());
}
