use asupersync::Cx;
use asupersync::http::h1::{HttpClient, HttpClientBuilder, Method};

use crate::constants::DEFAULT_USER_AGENT;
use crate::credentials::AccessCredentials;
use crate::error::{SimplefinError, Result};
use crate::models::AccountSet;

/// Parameters for querying accounts from the SimpleFIN server.
#[derive(Debug, Default)]
pub struct AccountQueryParams {
    /// Include transactions on or after this date (epoch seconds).
    pub start_date: Option<i64>,
    /// Include transactions before this date (epoch seconds).
    pub end_date: Option<i64>,
    /// Include pending transactions when supported.
    pub include_pending: bool,
    /// Filter to specific account IDs.
    pub account_ids: Option<Vec<String>>,
    /// When true, skip transactions and return balances only.
    pub balances_only: bool,
}

/// Client that uses SimpleFIN access credentials to retrieve account data.
pub struct AccessClient {
    credentials: AccessCredentials,
    http: HttpClient,
}

impl AccessClient {
    /// Creates a client that issues requests with the provided credentials.
    pub fn new(credentials: AccessCredentials, user_agent: Option<&str>) -> Self {
        let user_agent = user_agent.unwrap_or(DEFAULT_USER_AGENT);
        let http = HttpClientBuilder::new()
            .user_agent(user_agent)
            .build();
        AccessClient { credentials, http }
    }

    /// Retrieves account and transaction data from the SimpleFIN server.
    pub async fn get_accounts(
        &self,
        cx: &Cx,
        params: &AccountQueryParams,
    ) -> Result<AccountSet> {
        // Validate date range
        if let (Some(start), Some(end)) = (params.start_date, params.end_date)
            && start > end
        {
            return Err(SimplefinError::InvalidArgument(format!(
                "start_date ({start}) must be before or equal to end_date ({end})"
            )));
        }

        let query_params = self.build_query_params(params);
        let query_refs: Vec<(&str, &str)> = query_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let url = self.credentials.endpoint_url(
            &["accounts"],
            if query_refs.is_empty() { None } else { Some(&query_refs) },
        );
        let url_str = url.to_string();

        let auth_header = self.credentials.basic_auth_header_value();

        let response = self.http.request(
            cx,
            Method::Get,
            &url_str,
            vec![
                ("Accept".into(), "application/json".into()),
                ("Authorization".into(), auth_header),
            ],
            vec![],
        ).await?;

        if response.status != 200 {
            let body = super::body_as_string(&response.body);
            return Err(SimplefinError::Api {
                uri: url_str,
                status_code: response.status,
                message: "failed to fetch accounts".into(),
                response_body: body,
            });
        }

        let account_set: AccountSet =
            serde_json::from_slice(&response.body).map_err(|e| {
                SimplefinError::DataFormat {
                    message: "failed to parse accounts response".into(),
                    source: Some(Box::new(e)),
                }
            })?;

        Ok(account_set)
    }

    fn build_query_params(&self, params: &AccountQueryParams) -> Vec<(String, String)> {
        let mut query = Vec::new();

        if let Some(start) = params.start_date {
            query.push(("start-date".into(), start.to_string()));
        }
        if let Some(end) = params.end_date {
            query.push(("end-date".into(), end.to_string()));
        }
        if params.include_pending {
            query.push(("pending".into(), "1".into()));
        }
        if params.balances_only {
            query.push(("balances-only".into(), "1".into()));
        }
        if let Some(ref ids) = params.account_ids {
            for id in ids.iter().filter(|id| !id.is_empty()) {
                query.push(("account".into(), id.clone()));
            }
        }

        query
    }
}
