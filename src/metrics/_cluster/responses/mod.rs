use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
pub(crate) struct CluserHealthResponse(Value);

impl CluserHealthResponse {
    /// Inject labels into nodes response
    pub(crate) fn into_value(mut self, value_mangle: fn(&mut Map<String, Value>)) -> Value {
        if let Some(mut map) = self.0.as_object_mut() {
            value_mangle(&mut map)
        }

        self.0
    }
}
