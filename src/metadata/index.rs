use elasticsearch::cat::CatIndicesParts;
use elasticsearch::params::Time;
use elasticsearch::{Elasticsearch, Error};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::{exporter_metrics::SUBSYSTEM_REQ_HISTOGRAM, Exporter};

pub(crate) type IndexToMetadata = RwLock<IndexDataMap>;

pub(crate) type IndexDataMap = HashMap<String, IndexMetadata>;

/// Current Index metadata
#[derive(Debug, Clone)]
pub struct IndexMetadata {
    pub last_hearbeat: chrono::DateTime<chrono::Utc>,
}

impl Default for IndexMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexMetadata {
    /// Initialize Index metadata
    pub fn new() -> Self {
        Self {
            last_hearbeat: chrono::Utc::now(),
        }
    }

    /// Set heartbeat
    pub fn reset_heartbeat(&mut self) -> &mut Self {
        self.last_hearbeat = chrono::Utc::now();
        self
    }
}

/// Current Index metadata
#[derive(Debug, Deserialize, Default)]
pub(crate) struct CatIndexData {
    /// index name
    pub(crate) index: String,
}

pub(crate) async fn build(client: &Elasticsearch) -> Result<IndexDataMap, Error> {
    info!("Elasticsearch: fetching indices metadata");
    let index_data_map = client
        .cat()
        .indices(CatIndicesParts::None)
        .format("json")
        .h(&["index"])
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .time(Time::Ms)
        .send()
        .await?
        .json::<Vec<CatIndexData>>()
        .await?
        .into_iter()
        .map(|value| (value.index, IndexMetadata::new()))
        .collect();

    Ok(index_data_map)
}

#[inline]
fn update_map(old: &mut IndexDataMap, new: IndexDataMap) {
    for (key, new_metadata) in new {
        let _ = old.entry(key).or_insert(new_metadata).reset_heartbeat();
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
            .with_label_values(&["/_cat/indices", exporter.cluster_name()])
            .start_timer();

        match build(exporter.client()).await {
            Ok(new_metadata) => {
                update_map(&mut *exporter.index_metadata().write().await, new_metadata)
            }
            Err(e) => {
                error!("poll metadata metrics err {}", e);
            }
        }

        timer.observe_duration();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_map() {
        let index_data: Vec<CatIndexData> =
            serde_json::from_str(include_str!("../tests/files/metadata_cat_indices.json"))
                .expect("valid json");

        let mut old = index_data
            .into_iter()
            .map(|value| (value.index, IndexMetadata::new()))
            .collect::<IndexDataMap>();

        let key = "not_items_20210803115035";
        let last_hearbeat = old.get(key).unwrap().last_hearbeat;

        let new = old.clone();

        update_map(&mut old, new);

        // Check that heartbeat is updated to the latest value
        assert_ne!(last_hearbeat, old.get(key).unwrap().last_hearbeat);
    }
}
