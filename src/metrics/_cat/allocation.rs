use elasticsearch::cat::CatAllocationParts;
use elasticsearch::params::Bytes;

const SUBSYSTEM: &'static str = "cat_allocation";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client
        .cat()
        .allocation(CatAllocationParts::None)
        .format("json")
        .h(&["*"])
        .bytes(Bytes::B)
        .request_timeout(exporter.options.elasticsearch_global_timeout)
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .send()
        .await?;

    Ok(metric::from_values(response.json::<Vec<Value>>().await?))
}

crate::poll_metrics!();
