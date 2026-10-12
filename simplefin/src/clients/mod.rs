//! Native HTTP clients over asupersync. Not available on wasm32, where a
//! host transport sends the [`HttpRequestSpec`](crate::HttpRequestSpec)s
//! from [`crate::requests`] instead.

pub mod access_client;
pub mod bridge_client;

pub use access_client::{AccessClient, AccessFetcher};
pub use bridge_client::BridgeClient;

use asupersync::Cx;
use asupersync::http::h1::{HttpClient, HttpClientBuilder, Method, Response};

use crate::constants::DEFAULT_USER_AGENT;
use crate::error::{Result, SimplefinError};
use crate::requests::HttpRequestSpec;

/// An asupersync HTTP client sending `user_agent` (or the default).
fn http_client(user_agent: Option<&str>) -> HttpClient {
    HttpClientBuilder::new()
        .user_agent(user_agent.unwrap_or(DEFAULT_USER_AGENT))
        .build()
}

/// Sends `spec` (with an empty body) over `http`.
async fn send(http: &HttpClient, cx: &Cx, spec: HttpRequestSpec) -> Result<Response> {
    let method = Method::from_bytes(spec.method.as_bytes()).ok_or_else(|| {
        SimplefinError::InvalidArgument(format!("invalid HTTP method: {}", spec.method))
    })?;
    Ok(http
        .request(cx, method, &spec.url, spec.headers, Vec::new())
        .await?)
}
