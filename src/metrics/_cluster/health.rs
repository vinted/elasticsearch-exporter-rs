use prometheus::IntGaugeVec;
use serde_json::Map as SerdeMap;

use super::responses::CluserHealthResponse;
use elasticsearch::cluster::ClusterHealthParts;

pub(crate) const SUBSYSTEM: &str = "cluster_health";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cluster()
        .health(ClusterHealthParts::None)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .send()
        .await?;

    let values = response.json::<CluserHealthResponse>().await?.into_value();

    update_health_metrics_from_value(&values, exporter.metrics().cluster_health_status.clone());

    Ok(metric::from_value(values))
}

const COLORS: [&str; 3] = ["red", "green", "yellow"];

fn update_health_metrics_from_value(value: &Value, health_metric: IntGaugeVec) {
    if let Some(map) = value.as_object() {
        update_health_metrics(map, health_metric)
    }
}

// elasticsearch_cluster_health_cluster_status{cluster="some",status="green"} 1
fn update_health_metrics(map: &SerdeMap<String, Value>, health_metric: IntGaugeVec) {
    let cluster_status: String = map
        .get("status")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "red".into());

    let cluster_name: String = map
        .get("cluster_name")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .expect("/_cluster/health must contain cluster_name");

    for color in COLORS.iter() {
        if color == &cluster_status {
            health_metric
                .with_label_values(&[&cluster_name, &cluster_status])
                .set(1);
        } else {
            health_metric
                .with_label_values(&[&cluster_name, color])
                .set(0);
        }
    }
}

crate::poll_metrics!();

#[tokio::test]
async fn test_cluster_health() {
    let cluster_health: CluserHealthResponse =
        serde_json::from_str(include_str!("../../tests/files/cluster_health.json"))
            .expect("valid json");

    let values = cluster_health.into_value();

    let metrics = metric::from_value(values);
    assert!(!metrics.is_empty());

    let cluster_status_injection = metrics[0].iter().find(|m| m.key() == "status");
    assert!(cluster_status_injection.is_some());
}
