use elasticsearch::cluster::ClusterHealthParts;

pub(crate) const SUBSYSTEM: &str = "cluster_health";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cluster()
        .health(ClusterHealthParts::None)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .send()
        .await?;

    Ok(metric::from_value(response.json::<Value>().await?))
}

crate::poll_metrics!();
