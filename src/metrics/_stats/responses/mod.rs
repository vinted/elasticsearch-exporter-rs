use serde_json::Value;
use std::collections::HashMap;

/// Nodes response
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StatsResponse {
    _shards: HashMap<String, Value>,
    _all: HashMap<String, Value>,
    indices: HashMap<String, Value>,
}

impl StatsResponse {
    /// Inject labels into nodes response
    pub(crate) async fn into_values(mut self, keys_to_remove: &[&'static str]) -> Vec<Value> {
        let mut values: Vec<Value> = Vec::new();

        // Inject node label
        for (index_name, mut data) in self.indices.drain() {
            inject_index(&mut data, &index_name, keys_to_remove);

            values.push(data);
        }

        for (_, data) in self._all.drain() {
            values.push(data);
        }

        for (_, data) in self._shards.drain() {
            values.push(data);
        }

        values
    }
}

fn inject_index(value: &mut Value, index_name: &str, keys_to_remove: &[&'static str]) {
    if let Some(map) = value.as_object_mut() {
        let _ = map.insert("index".into(), Value::String(index_name.to_string()));

        // Doing inverse removal because serde_json::Map does not have .retain
        for to_remove in keys_to_remove {
            let _ = map.remove(*to_remove);
        }

        for (_, object_value) in map {
            inject_index(object_value, index_name, keys_to_remove);
        }
    }
    if let Some(array) = value.as_array_mut() {
        for object_array in array {
            inject_index(object_array, index_name, keys_to_remove);
        }
    }
}
