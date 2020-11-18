use elasticsearch::nodes::NodesUsageParts;
use std::collections::HashMap;

pub(crate) const SUBSYSTEM: &'static str = "nodes_usage";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client
        .nodes()
        .usage(NodesUsageParts::None)
        .request_timeout(exporter.options.elasticsearch_global_timeout)
        .send()
        .await?;

    let values = response
        .json::<NodesUsage>()
        .await?
        .inject_labels(&exporter.id_to_name);

    Ok(metric::from_values(values))
}

crate::poll_metrics!();

#[derive(Debug, Serialize, Deserialize)]
struct NodesUsage {
    nodes: HashMap<String, HashMap<String, Value>>,
}

pub(crate) const RELEVANT_METRICS: &[&'static str; 2] = &["rest_actions", "aggregations"];

impl NodesUsage {
    fn inject_labels(mut self, labels: &HashMap<String, String>) -> Vec<Value> {
        let mut values: Vec<Value> = Vec::new();

        // Inject node label
        for (node, mut data) in self.nodes.drain() {
            if let Some(label) = labels.get(&node) {
                for (k, mut v) in data.drain() {
                    if RELEVANT_METRICS.contains(&k.as_str()) {
                        if let Some(object) = v.as_object_mut() {
                            if !object.is_empty() {
                                let _ = object.insert("name".into(), Value::String(label.clone()));
                                let _ = object.insert("usage".into(), Value::String(k));

                                // Only push data objects that were annotated with data
                                values.push(v);
                            }
                        }
                    }
                }
            }
        }

        values
    }
}

#[test]
fn test_inject_labels() {
    let usage: NodesUsage =
        serde_json::from_str(include_str!("../../tests/files/nodes_usage.json"))
            .expect("valid json");

    let expected_name: String = "m1-nodename.example.com".into();

    let mut labels: HashMap<String, String> = HashMap::new();
    let _ = labels.insert("U-WnGaTpRxucgde3miiDWw".into(), expected_name.clone());

    let values = usage.inject_labels(&labels);
    assert!(!values.is_empty());

    let aggregations = values[0].as_object().unwrap();
    assert_eq!(
        aggregations.get("name").unwrap().as_str().unwrap(),
        expected_name
    );
    assert!(RELEVANT_METRICS.contains(&aggregations.get("usage").unwrap().as_str().unwrap()));
}
