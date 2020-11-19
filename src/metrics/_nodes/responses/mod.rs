use serde_json::Value;
use std::collections::HashMap;

use crate::metadata::{IdToMetadata, NodeData};

/// Nodes response
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NodesResponse {
    nodes: HashMap<String, Value>,
}

impl NodesResponse {
    /// Inject labels into nodes response
    pub(crate) async fn into_values(
        mut self,
        metadata: &IdToMetadata,
        keys_to_remove: &[&'static str],
    ) -> Vec<Value> {
        let mut values: Vec<Value> = Vec::new();

        let metadata_read = metadata.read().await;

        // Inject node label
        for (node_id, mut data) in self.nodes.drain() {
            if let Some(node_metadata) = metadata_read.get(&node_id) {
                inject_label(&mut data, &node_metadata, keys_to_remove);

                values.push(data);
            }
        }

        values
    }
}

fn inject_label(value: &mut Value, node_data: &NodeData, keys_to_remove: &[&'static str]) {
    if let Some(map) = value.as_object_mut() {
        let _ = map.insert("name".into(), Value::String(node_data.name.to_string()));
        let _ = map.insert(
            "cluster_version".into(),
            Value::String(node_data.version.to_string()),
        );

        // Doing inverse removal because serde_json::Map does not have .retain
        for to_remove in keys_to_remove {
            let _ = map.remove(*to_remove);
        }

        for (_, object_value) in map {
            inject_label(object_value, node_data, keys_to_remove);
        }
    }
    if let Some(array) = value.as_array_mut() {
        for object_array in array {
            inject_label(object_array, node_data, keys_to_remove);
        }
    }
}
