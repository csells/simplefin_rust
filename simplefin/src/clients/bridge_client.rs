use asupersync::Cx;
use asupersync::http::h1::HttpClient;

use crate::constants::DEFAULT_BRIDGE_ROOT_URL;
use crate::credentials::{AccessCredentials, SetupToken};
use crate::error::{Result, SimplefinError};
use crate::models::BridgeInfo;
use crate::requests::{HttpRequestSpec, api_error, parse_claim_response_from};

/// HTTP client for interacting with a SimpleFIN Bridge server.
pub struct BridgeClient {
    root: String,
    http: HttpClient,
}

impl BridgeClient {
    /// Creates a client targeting the provided bridge root URL.
    pub fn new(root: Option<&str>, user_agent: Option<&str>) -> Self {
        BridgeClient {
            root: root.unwrap_or(DEFAULT_BRIDGE_ROOT_URL).to_string(),
            http: super::http_client(user_agent),
        }
    }

    /// Retrieves the list of protocol versions supported by the bridge.
    pub async fn get_info(&self, cx: &Cx) -> Result<BridgeInfo> {
        let url = format!("{}/info", self.root.trim_end_matches('/'));
        let spec = HttpRequestSpec {
            method: "GET",
            url: url.clone(),
            headers: vec![("Accept".into(), "application/json".into())],
        };
        let response = super::send(&self.http, cx, spec).await?;

        if response.status != 200 {
            return Err(api_error(
                &url,
                response.status,
                "failed to query bridge info",
                &response.body,
            ));
        }

        serde_json::from_slice(&response.body).map_err(|e| SimplefinError::DataFormat {
            message: "failed to parse bridge info response".into(),
            source: Some(Box::new(e)),
        })
    }

    /// Exchanges a user-provided setup token for long-lived access credentials.
    pub async fn claim_access_credentials(
        &self,
        cx: &Cx,
        setup_token: &str,
    ) -> Result<AccessCredentials> {
        let spec = SetupToken::parse(setup_token)?.claim_request();
        let claim_url = spec.url.clone();
        let response = super::send(&self.http, cx, spec).await?;
        parse_claim_response_from(&claim_url, response.status, &response.body)
    }
}
