use elasticsearch::nodes::NodesStatsParts;

use super::responses::NodesResponse;

pub(crate) const SUBSYSTEM: &'static str = "nodes_stats";

// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-nodes-stats.html
async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client
        .nodes()
        .stats(NodesStatsParts::None)
        .request_timeout(exporter.options.elasticsearch_global_timeout)
        .send()
        .await?;

    let values = response
        .json::<NodesResponse>()
        .await?
        .inject_labels(&exporter.id_to_name);

    Ok(metric::from_values(values))
}

crate::poll_metrics!();

#[test]
fn test_inject_labels_nodes_stats() {
    use std::collections::HashMap;

    let usage: NodesResponse =
        serde_json::from_str(include_str!("../../tests/files/nodes_stats.json"))
            .expect("valid json");

    let expected_name: String = "m1-nodename.example.com".into();

    let mut labels: HashMap<String, String> = HashMap::new();
    let _ = labels.insert("U-WnGaTpRxucgde3miiDWw".into(), expected_name.clone());

    let values = usage.inject_labels(&labels);
    assert!(!values.is_empty());

    let value = values.last().unwrap().as_object().unwrap();
    assert_eq!(value.get("name").unwrap().as_str().unwrap(), expected_name);
}
