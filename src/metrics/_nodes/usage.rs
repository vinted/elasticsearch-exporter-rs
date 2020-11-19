use elasticsearch::nodes::NodesUsageParts;

use super::responses::NodesResponse;

pub(crate) const SUBSYSTEM: &'static str = "nodes_usage";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .nodes()
        .usage(NodesUsageParts::None)
        .request_timeout(exporter.options().elasticsearch_global_timeout)
        .send()
        .await?;

    let values = response
        .json::<NodesResponse>()
        .await?
        .into_values(exporter.metadata(), REMOVE_KEYS);

    Ok(metric::from_values(values))
}

const REMOVE_KEYS: &[&'static str; 2] = &["since", "timestamp"];

crate::poll_metrics!();

#[test]
fn test_nodes_usage() {
    use std::collections::HashMap;

    let usage: NodesResponse =
        serde_json::from_str(include_str!("../../tests/files/nodes_usage.json"))
            .expect("valid json");

    let expected_name: String = "m1-nodename.example.com".into();

    let mut labels: HashMap<String, String> = HashMap::new();
    let _ = labels.insert("U-WnGaTpRxucgde3miiDWw".into(), expected_name.clone());

    let values = usage.into_values(&labels, &["timestamp"]);
    assert!(!values.is_empty());

    let aggregations = values[0].as_object().unwrap();
    assert_eq!(
        aggregations.get("name").unwrap().as_str().unwrap(),
        expected_name
    );
    assert!(aggregations.get("timestamp").is_none());
}
