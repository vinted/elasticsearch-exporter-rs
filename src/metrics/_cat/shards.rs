use elasticsearch::cat::CatShardsParts;
use elasticsearch::params::{Bytes, Time};

use super::responses::CatResponse;

pub(crate) const SUBSYSTEM: &str = "cat_shards";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let filter_path = exporter
        .options()
        .query_filter_path_for_subsystem(SUBSYSTEM);

    let cat = exporter.client().cat();

    let mut shards_stats = cat
        .shards(CatShardsParts::Index(&["*"]))
        .format("json")
        .h(&["*"])
        .bytes(Bytes::B)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .time(Time::Ms);

    if !filter_path.is_empty() {
        shards_stats = shards_stats.filter_path(&filter_path)
    }

    let response = shards_stats.send().await?;

    let values = response.json::<CatResponse>().await?.into_values(|map| {
        if map
            .get("state")
            .map(|state| state == "RELOCATING")
            .unwrap_or(false)
        {
            map.clear();
        }
    });

    Ok(metric::from_values(values))
}

crate::poll_metrics!();

#[tokio::test]
async fn test_cat_shards() {
    let cat: CatResponse = serde_json::from_str(include_str!("../../tests/files/cat_shards.json"))
        .expect("valid json");

    let got = cat.into_values(|map| {
        let state: bool = map
            .get("state")
            .map(|state| state == "RELOCATING")
            .unwrap_or(false);

        if state {
            map.clear();
        }
    });

    assert_eq!(got.len(), 1, "cat_shards got {:?}", got);
}
