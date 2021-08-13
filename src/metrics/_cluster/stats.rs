use elasticsearch::cluster::ClusterStatsParts;

pub(crate) const SUBSYSTEM: &str = "cluster_stats";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cluster()
        .stats(ClusterStatsParts::None)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .send()
        .await?;

    Ok(metric::from_value(response.json::<Value>().await?))
}

crate::poll_metrics!();
