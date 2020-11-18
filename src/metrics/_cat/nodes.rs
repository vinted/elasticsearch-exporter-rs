use elasticsearch::params::Bytes;

pub(crate) const SUBSYSTEM: &'static str = "cat_nodes";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client
        .cat()
        .nodes()
        .format("json")
        .h(&["*"])
        .bytes(Bytes::B)
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .request_timeout(exporter.options.elasticsearch_global_timeout)
        .send()
        .await?;

    Ok(metric::from_values(response.json::<Vec<Value>>().await?))
}

crate::poll_metrics!();
