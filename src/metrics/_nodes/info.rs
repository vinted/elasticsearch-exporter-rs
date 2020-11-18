use elasticsearch::nodes::NodesInfoParts;

use super::responses::NodesResponse;

pub(crate) const SUBSYSTEM: &'static str = "nodes_info";

// https://www.elastic.co/guide/en/elasticsearch/reference/current/cluster-nodes-stats.html
async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client
        .nodes()
        .info(NodesInfoParts::None)
        // TODO: exclude by metric
        .request_timeout(exporter.options.elasticsearch_global_timeout)
        .send()
        .await?;

    let values = response
        .json::<NodesResponse>()
        .await?
        .into_values(&exporter.id_to_name, REMOVE_KEYS);

    Ok(metric::from_values(values))
}

const REMOVE_KEYS: &[&'static str; 21] = &[
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
    "memory_lock",
    "x-pack",
    "strategy",
    "default",
    "pidfile",
    "path",
    "settings",
];

crate::poll_metrics!();

#[test]
fn test_nodes_info() {
    use std::collections::HashMap;

    let usage: NodesResponse =
        serde_json::from_str(include_str!("../../tests/files/nodes_info.json"))
            .expect("valid json");

    let expected_name: String = "m1-nodename.example.com".into();

    let mut labels: HashMap<String, String> = HashMap::new();
    let _ = labels.insert("U-WnGaTpRxucgde3miiDWw".into(), expected_name.clone());

    let values = usage.into_values(&labels, &["timestamp"]);
    assert!(!values.is_empty());

    let value = values[0].as_object().unwrap();
    assert_eq!(value.get("name").unwrap().as_str().unwrap(), expected_name);
    assert!(value.get("timestamp").is_none());
}
