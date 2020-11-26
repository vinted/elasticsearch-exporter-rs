use serde_json::{Map, Value};

#[derive(Deserialize, Debug)]
pub(crate) struct CatResponse(Vec<Value>);

impl CatResponse {
    /// Inject labels into nodes response
    pub(crate) fn into_values(mut self, value_mangle: fn(&mut Map<String, Value>)) -> Vec<Value> {
        for value in self.0.iter_mut() {
            if let Some(mut map) = value.as_object_mut() {
                value_mangle(&mut map)
            }
        }

        // Cleanup empty values possibly mangled by `value_mangle` closure
        self.0
            .retain(|value| value.as_object().map(|map| !map.is_empty()).unwrap_or(true));

        self.0
    }
}
