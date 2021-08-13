use elasticsearch::cat::CatThreadPoolParts;

pub(crate) const SUBSYSTEM: &str = "cat_thread_pool";

async fn metrics(exporter: &Exporter) -> Result<Vec<Metrics>, elasticsearch::Error> {
    let response = exporter
        .client()
        .cat()
        .thread_pool(CatThreadPoolParts::ThreadPoolPatterns(&["*"]))
        .format("json")
        .h(&["*"])
        // Return local information, do not retrieve the state from master node (default: false)
        .local(true)
        .request_timeout(exporter.options().timeout_for_subsystem(SUBSYSTEM))
        .send()
        .await?;

    Ok(metric::from_values(response.json::<Vec<Value>>().await?))
}

crate::poll_metrics!();
