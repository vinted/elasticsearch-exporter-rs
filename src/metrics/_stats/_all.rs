use elasticsearch::indices::IndicesStatsParts;

use super::responses::StatsResponse;

pub(crate) const SUBSYSTEM: &str = "stats";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let fields = exporter.options().query_fields_for_subsystem(SUBSYSTEM);

    let indices = exporter.client().indices();

    let mut indices_stats = indices
        .stats(IndicesStatsParts::None)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM));

    if !fields.is_empty() {
        indices_stats = indices_stats.fields(&fields);
    }

    let response = indices_stats.send().await?;

    let values = response
        .json::<StatsResponse>()
        .await?
        .into_values(REMOVE_KEYS)
        .await;

    Ok(metric::from_values(values))
}

const REMOVE_KEYS: &[&str] = &["uuid"];

crate::poll_metrics!();

#[tokio::test]
async fn test_global_stats() {
    let stats: StatsResponse =
        serde_json::from_str(include_str!("../../tests/files/_stats.json")).expect("valid json");

    let values = stats.into_values(&["timestamp"]).await;
    assert!(!values.is_empty());

    let expected_name: String = "generic-index-name".into();
    assert_eq!(
        values[0].get("index").unwrap().as_str().unwrap(),
        expected_name
    );
}
