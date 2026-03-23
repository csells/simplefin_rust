use asupersync::Cx;
use asupersync::http::h1::{HttpClient, HttpClientBuilder, Method};

use crate::constants::{DEFAULT_BRIDGE_ROOT_URL, DEFAULT_USER_AGENT};
use crate::credentials::{AccessCredentials, SetupToken};
use crate::error::{SimplefinError, Result};
use crate::models::BridgeInfo;

/// HTTP client for interacting with a SimpleFIN Bridge server.
pub struct BridgeClient {
    root: String,
    http: HttpClient,
}

impl BridgeClient {
    /// Creates a client targeting the provided bridge root URL.
    pub fn new(root: Option<&str>, user_agent: Option<&str>) -> Self {
        let root = root.unwrap_or(DEFAULT_BRIDGE_ROOT_URL).to_string();
        let user_agent = user_agent.unwrap_or(DEFAULT_USER_AGENT);
        let http = HttpClientBuilder::new()
            .user_agent(user_agent)
            .build();
        BridgeClient { root, http }
    }

    /// Retrieves the list of protocol versions supported by the bridge.
    pub async fn get_info(&self, cx: &Cx) -> Result<BridgeInfo> {
        let url = format!("{}/info", self.root.trim_end_matches('/'));

        let response = self.http.request(
            cx,
            Method::Get,
            &url,
            vec![
                ("Accept".into(), "application/json".into()),
            ],
            vec![],
        ).await?;

        if response.status != 200 {
            let body = super::body_as_string(&response.body);
            return Err(SimplefinError::Api {
                uri: url,
                status_code: response.status,
                message: "failed to query bridge info".into(),
                response_body: body,
            });
        }

        let info: BridgeInfo = serde_json::from_slice(&response.body).map_err(|e| {
            SimplefinError::DataFormat {
                message: "failed to parse bridge info response".into(),
                source: Some(Box::new(e)),
            }
        })?;

        Ok(info)
    }

    /// Exchanges a user-provided setup token for long-lived access credentials.
    pub async fn claim_access_credentials(
        &self,
        cx: &Cx,
        setup_token: &str,
    ) -> Result<AccessCredentials> {
        let parsed_token = SetupToken::parse(setup_token)?;
        let claim_url = parsed_token.claim_url.to_string();

        let response = self.http.request(
            cx,
            Method::Post,
            &claim_url,
            vec![
                ("Accept".into(), "text/plain".into()),
            ],
            vec![],
        ).await?;

        if response.status != 200 {
            let body = super::body_as_string(&response.body);
            return Err(SimplefinError::Api {
                uri: claim_url,
                status_code: response.status,
                message: "failed to claim access URL".into(),
                response_body: body,
            });
        }

        let body = String::from_utf8_lossy(&response.body).trim().to_string();
        if body.is_empty() {
            return Err(SimplefinError::Api {
                uri: claim_url,
                status_code: response.status,
                message: "claim response did not include an access URL".into(),
                response_body: String::new(),
            });
        }

        AccessCredentials::parse(&body)
    }
}
