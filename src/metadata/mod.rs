use elasticsearch::cluster::ClusterHealthParts;
use elasticsearch::nodes::NodesInfoParts;
use elasticsearch::{Elasticsearch, Error};
use serde_json::Value;
use std::collections::HashMap;

type NodeIdToName = HashMap<String, String>;

pub(crate) async fn nodes_id_map(client: &Elasticsearch) -> Result<NodeIdToName, Error> {
    let nodes_os = client
        .nodes()
        .info(NodesInfoParts::None)
        // TODO: fix/report bug for filter_path
        // DEBUG reqwest::async_impl::client > response '200 OK' for http://supernode:9200/_nodes?filter_path=os
        // ERROR elasticsearch_exporter      > error decoding response body: missing field `nodes` at line 1 column 2
        // .filter_path(&["os"])
        .send()
        .await?
        .json::<NodesOs>()
        .await?;

    Ok(nodes_os.into())
}

#[derive(Debug, Serialize, Deserialize)]
struct NodesOs {
    nodes: HashMap<String, NodeData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeData {
    name: String,
}

impl From<NodesOs> for HashMap<String, String> {
    fn from(node_os: NodesOs) -> Self {
        let mut map = NodeIdToName::new();

        for (k, v) in node_os.nodes.into_iter() {
            let _ = map.insert(k, v.name);
        }

        map
    }
}

pub(crate) async fn cluster_name(client: &Elasticsearch) -> Result<String, Error> {
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
