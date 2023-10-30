use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct CluserHealthResponse(Value);

impl CluserHealthResponse {
    /// Inject labels into nodes response
    pub(crate) fn into_value(self) -> Value {
        self.0
    }
}
