use elasticsearch::cluster::ClusterHealthParts;
use elasticsearch::{Elasticsearch, Error};
use serde_json::Value;

pub(crate) mod node_data;
pub(crate) use node_data::{build, poll, IdToMetadata, NodeData};

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
