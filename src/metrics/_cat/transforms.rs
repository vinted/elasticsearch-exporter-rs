use elasticsearch::cat::CatTransformsParts;
use elasticsearch::params::Time;

pub(crate) const SUBSYSTEM: &str = "cat_transforms";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cat()
        .transforms(CatTransformsParts::TransformId("*"))
        .format("json")
        .h(&["*"])
        .time(Time::Ms)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .send()
        .await?;

    Ok(metric::from_values(response.json::<Vec<Value>>().await?))
}

crate::poll_metrics!();
