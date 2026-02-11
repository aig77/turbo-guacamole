use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::Instrument;
use uuid::Uuid;

pub async fn tracing_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();

    // Extract or generate request_id
    let request_id = request
        .headers()
        .get("X-Request-ID")
        .and_then(|h| h.to_str().ok())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();

    // Create span with all relevant fields
    let span = tracing::info_span!(
        "request",
        method = %method,
        uri = %uri,
        version = ?version,
        request_id = %request_id,
    );

    // Enter the span and log within it
    let _guard = span.enter();
    tracing::info!("started processing request");
    drop(_guard);

    // Run request through remaining middleware/handlers
    let mut response = next.run(request).instrument(span.clone()).await;

    // Log completion within the span
    let _guard = span.enter();
    let latency = start.elapsed();
    let status = response.status();
    tracing::info!(
        status = %status,
        latency = ?latency,
        "finished processing request"
    );
    drop(_guard);

    // Add request_id to response headers
    response
        .headers_mut()
        .insert("X-Request-ID", request_id.parse().unwrap());

    response
}
