use elasticsearch::cat::CatRecoveryParts;
use elasticsearch::params::{Bytes, Time};

pub(crate) const SUBSYSTEM: &'static str = "cat_recovery";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cat()
        .recovery(CatRecoveryParts::Index(&["*"]))
        .format("json")
        .h(&["*"])
        .bytes(Bytes::B)
        .time(Time::Ms)
        .request_timeout(exporter.options().elasticsearch_global_timeout)
        .send()
        .await?;

    Ok(metric::from_values(response.json::<Vec<Value>>().await?))
}

crate::poll_metrics!();
