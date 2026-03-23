use serde::{Deserialize, Serialize};

/// Metadata describing the capabilities of a SimpleFIN Bridge server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeInfo {
    /// Supported SimpleFIN protocol versions reported by the bridge.
    pub versions: Vec<String>,
}
