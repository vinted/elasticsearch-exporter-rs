use prometheus::HistogramVec;

lazy_static! {
    /// Subsystem requests histogram
    pub static ref SUBSYSTEM_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "elasticsearch_subsystem_request_duration_seconds",
        "The Elasticsearch subsystem request latencies in seconds.",
        &["subsystem", "cluster"]
    )
    .expect("valid histogram vec metric");
}
