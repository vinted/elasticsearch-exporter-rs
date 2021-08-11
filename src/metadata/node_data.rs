use elasticsearch::nodes::NodesInfoParts;
use elasticsearch::{Elasticsearch, Error};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::{exporter_metrics::SUBSYSTEM_REQ_HISTOGRAM, Exporter};

pub(crate) type IdToMetadata = RwLock<NodeDataMap>;

pub(crate) type NodeDataMap = HashMap<String, NodeData>;

pub(crate) async fn build(client: &Elasticsearch) -> Result<IdToMetadata, Error> {
    Ok(RwLock::new(_build(client).await?))
}

pub(crate) async fn _build(client: &Elasticsearch) -> Result<NodeDataMap, Error> {
    info!("Elasticsearch: fetching cluster metadata");
    let nodes_os = client
        .nodes()
        .info(NodesInfoParts::Metric(&["os"]))
        .send()
        .await?
        .json::<NodesOs>()
        .await?;

    Ok(nodes_os.nodes)
}

#[derive(Debug, Deserialize)]
struct NodesOs {
    nodes: HashMap<String, NodeData>,
}

/// Node metadata
#[derive(Debug, Deserialize, Default)]
pub struct NodeData {
    /// Node FQDN
    pub name: String,
    /// IP
    pub ip: String,
    /// Node Elasticsearch version
    pub version: String,
}

#[inline]
fn update_map(old: &mut NodeDataMap, new: NodeDataMap) {
    for (key, new_metadata) in new.into_iter() {
        let _ = old.insert(key, new_metadata);
    }
}

#[allow(unused)]
pub(crate) async fn poll(exporter: Exporter) {
    let start = tokio::time::Instant::now();

    let mut interval =
        tokio::time::interval_at(start, exporter.options().exporter_metadata_refresh_interval);

    loop {
        let _ = interval.tick().await;

        let timer = SUBSYSTEM_REQ_HISTOGRAM
            .with_label_values(&["/_nodes/os", exporter.cluster_name()])
            .start_timer();

        match _build(&exporter.client()).await {
            Ok(new_metadata) => update_map(&mut *exporter.metadata().write().await, new_metadata),
            Err(e) => {
                error!("poll metadata metrics err {}", e);
            }
        }

        timer.observe_duration();
    }
}

#[test]
fn test_update_map() {
    let test_key = "testtsss".to_string();
    let test_ip = "0.0.0.0".to_string();

    let mut old = NodeDataMap::new();
    let _ = old.insert(
        test_key.clone(),
        NodeData {
            name: "name.fqdn.com".into(),
            ip: test_ip.clone(),
            version: "7.7.0".into(),
        },
    );

    let mut new = NodeDataMap::new();
    let _ = new.insert(
        test_key.clone(),
        NodeData {
            name: "name.fqdn.com".into(),
            ip: test_ip.clone(),
            version: "7.9.3".into(),
        },
    );

    update_map(&mut old, new);

    // Version is updated
    assert_eq!(old.get(&test_key).unwrap().version, "7.9.3");
}
