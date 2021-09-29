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

    let values = response
        .json::<CluserHealthResponse>()
        .await?
        .into_value(inject_cluster_health);

    Ok(metric::from_value(values))
}

lazy_static! {
    static ref CLUSTER_STATUS: IntGaugeVec = register_int_gauge_vec!(
        "elasticsearch_cluster_health_status",
        "Whether all primary and replica shards are allocated.",
        &["cluster", "color"]
    )
    .expect("valid prometheus metric");
}

const COLORS: [&str; 3] = ["red", "green", "yellow"];

// elasticsearch_cluster_health_cluster_status{cluster="some",status="green"} 1
fn inject_cluster_health(map: &mut SerdeMap<String, Value>) {
    let cluster_status: String = map
        .get("status")
        .map(|v| v.as_str())
        .flatten()
        .map(ToOwned::to_owned)
        .unwrap_or("red".into());

    let cluster_name: String = map
        .get("cluster_name")
        .map(|v| v.as_str())
        .flatten()
        .map(ToOwned::to_owned)
        .expect("/_cluster/health must contain cluster_name");

    for color in COLORS.iter() {
        if color == &cluster_status {
            CLUSTER_STATUS
                .with_label_values(&[&cluster_name, &cluster_status])
                .set(1);
        } else {
            CLUSTER_STATUS
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

    let values = cluster_health.into_value(inject_cluster_health);

    let metrics = metric::from_value(values);
    assert!(!metrics.is_empty());

    let cluster_status_injection = metrics[0].iter().find(|m| m.key() == "cluster_status");
    assert!(cluster_status_injection.is_some());
}
