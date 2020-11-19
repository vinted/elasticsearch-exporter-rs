use elasticsearch::cluster::ClusterHealthParts;
use elasticsearch::nodes::NodesInfoParts;
use elasticsearch::{Elasticsearch, Error};
use serde_json::Value;
use std::collections::HashMap;

pub(crate) type IdToMetadata = HashMap<String, NodeData>;

// TODO: add `version` label
// TODO: use RwLock
pub(crate) async fn build(client: &Elasticsearch) -> Result<IdToMetadata, Error> {
    info!("Elasticsearch: fetching cluster node ID's");
    let nodes_os = client
        .nodes()
        .info(NodesInfoParts::Metric(&["os"]))
        .send()
        .await?
        .json::<NodesOs>()
        .await?;

    Ok(nodes_os.nodes)
}

#[derive(Debug, Serialize, Deserialize)]
struct NodesOs {
    nodes: HashMap<String, NodeData>,
}

/// Node metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeData {
    /// Node FQDN
    pub name: String,
    /// IP
    pub ip: String,
    /// Node Elasticsearch version
    pub version: String,
}

pub(crate) async fn cluster_name(client: &Elasticsearch) -> Result<String, Error> {
    info!("Elasticsearch: fetching cluster_name");
    Ok(client
        .cluster()
        .health(ClusterHealthParts::None)
        .send()
        .await?
        .json::<Value>()
        .await?["cluster_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
}
