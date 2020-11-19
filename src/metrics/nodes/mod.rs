use elasticsearch::cat::NodesUsageParts;
use elasticsearch::params::{Bytes, Time};

const SUBSYSTEM: &'static str = "nodes_usage";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client
        .nodes()
        .usage(NodesUsageParts::None)
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
