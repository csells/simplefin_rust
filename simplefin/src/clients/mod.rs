pub mod access_client;
pub mod bridge_client;

pub use access_client::{AccessClient, AccountQueryParams};
pub use bridge_client::BridgeClient;

/// Converts a response body to a String, replacing invalid UTF-8 with the replacement character.
fn body_as_string(body: &[u8]) -> String {
    String::from_utf8_lossy(body).to_string()
}
