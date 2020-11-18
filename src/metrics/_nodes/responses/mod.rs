use serde_json::Value;
use std::collections::HashMap;

/// Nodes response
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NodesResponse {
    nodes: HashMap<String, Value>,
}

impl NodesResponse {
    /// Inject labels into nodes response
    pub(crate) fn inject_labels(mut self, labels: &HashMap<String, String>) -> Vec<Value> {
        let mut values: Vec<Value> = Vec::new();

        // Inject node label
        for (node, mut data) in self.nodes.drain() {
            if let Some(label) = labels.get(&node) {
                inject_label(&mut data, &label);

                values.push(data);
            }
        }

        values
    }
}

fn inject_label(value: &mut Value, name: &str) {
    if let Some(object) = value.as_object_mut() {
        let _ = object.insert("name".into(), Value::String(name.to_string()));

        for (_, object_value) in object {
            inject_label(object_value, name);
        }
    }
    if let Some(array) = value.as_array_mut() {
        for object_array in array {
            inject_label(object_array, name);
        }
    }
}
