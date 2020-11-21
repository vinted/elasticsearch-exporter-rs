use elasticsearch::indices::IndicesStatsParts;

use super::responses::StatsResponse;

pub(crate) const SUBSYSTEM: &'static str = "stats";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .indices()
        .stats(IndicesStatsParts::None)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .send()
        .await?;

    let values = response
        .json::<StatsResponse>()
        .await?
        .into_values(REMOVE_KEYS)
        .await;

    Ok(metric::from_values(values))
}

const REMOVE_KEYS: &[&'static str] = &[];

crate::poll_metrics!();
