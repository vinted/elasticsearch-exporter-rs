use elasticsearch::cat::CatIndicesParts;
use elasticsearch::params::{Bytes, Time};

const SUBSYSTEM: &'static str = "cat_indices";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client
        .cat()
        .indices(CatIndicesParts::Index(&["*"]))
        .format("json")
        .h(&["*"])
        .bytes(Bytes::B)
        .request_timeout(exporter.options.elasticsearch_global_timeout)
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .time(Time::Ms)
        .send()
        .await?;

    Ok(metric::from_values(response.json::<Vec<Value>>().await?))
}

crate::poll_metrics!();
