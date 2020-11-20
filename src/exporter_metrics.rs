use prometheus::HistogramVec;

lazy_static! {
    /// HTTP requests histogram
    pub static ref HTTP_REQ_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["handler"]
    )
    .expect("valid histogram vec metric");
}
