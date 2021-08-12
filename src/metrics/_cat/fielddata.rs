use elasticsearch::cat::CatFielddataParts;
use elasticsearch::params::Bytes;

pub(crate) const SUBSYSTEM: &str = "cat_fielddata";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cat()
        .fielddata(CatFielddataParts::Fields(&["*"]))
        .format("json")
        .h(&["*"])
        .bytes(Bytes::B)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .send()
        .await?;

    Ok(metric::from_values(response.json::<Vec<Value>>().await?))
}

crate::poll_metrics!();
