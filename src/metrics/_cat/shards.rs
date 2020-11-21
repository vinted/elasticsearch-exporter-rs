use elasticsearch::cat::CatShardsParts;
use elasticsearch::params::{Bytes, Time};

pub(crate) const SUBSYSTEM: &'static str = "cat_shards";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cat()
        .shards(CatShardsParts::Index(&["*"]))
        .format("json")
        .h(&["*"])
        .bytes(Bytes::B)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .time(Time::Ms)
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .send()
        .await?;

    Ok(metric::from_values(response.json::<Vec<Value>>().await?))
}

crate::poll_metrics!();
