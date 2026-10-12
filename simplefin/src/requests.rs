//! Transport-independent SimpleFIN HTTP requests and response parsing.
//!
//! These pure helpers describe each request as an [`HttpRequestSpec`] and
//! turn a status code and body back into typed results, so any transport
//! (asupersync natively, `fetch` in the browser) shares one definition of
//! the protocol. [`AccessClient`](crate::AccessClient) and
//! [`BridgeClient`](crate::BridgeClient) are built on them.

use crate::credentials::{AccessCredentials, SetupToken};
use crate::error::{Result, SimplefinError};
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

/// A transport-neutral HTTP request with an empty body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestSpec {
    /// `"GET"` or `"POST"`.
    pub method: &'static str,
    /// Request URL. Never embeds credentials; those travel in `headers`.
    pub url: String,
    /// Request headers, including `Authorization: Basic …` where needed.
    pub headers: Vec<(String, String)>,
}

impl AccessCredentials {
    /// The `GET /accounts` request for `params`, authenticated with Basic Auth.
    ///
    /// Fails with [`SimplefinError::InvalidArgument`] when `start_date` is
    /// after `end_date`.
    pub fn accounts_request(&self, params: &AccountQueryParams) -> Result<HttpRequestSpec> {
        if let (Some(start), Some(end)) = (params.start_date, params.end_date)
            && start > end
        {
            return Err(SimplefinError::InvalidArgument(format!(
                "start_date ({start}) must be before or equal to end_date ({end})"
            )));
        }

        let query = accounts_query(params);
        let query_refs: Vec<(&str, &str)> = query
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let url = self.endpoint_url(
            &["accounts"],
            if query_refs.is_empty() {
                None
            } else {
                Some(&query_refs)
            },
        );

        Ok(HttpRequestSpec {
            method: "GET",
            url: url.to_string(),
            headers: vec![
                ("Accept".into(), "application/json".into()),
                ("Authorization".into(), self.basic_auth_header_value()),
            ],
        })
    }
}

impl SetupToken {
    /// The one-time `POST` to the claim URL that returns the access URL.
    pub fn claim_request(&self) -> HttpRequestSpec {
        HttpRequestSpec {
            method: "POST",
            url: self.claim_url.to_string(),
            headers: vec![("Accept".into(), "text/plain".into())],
        }
    }
}

fn accounts_query(params: &AccountQueryParams) -> Vec<(String, String)> {
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

/// Parses an `/accounts` response. Non-200 statuses become
/// [`SimplefinError::Api`]; malformed JSON becomes [`SimplefinError::DataFormat`].
pub fn parse_accounts_response(status: u16, body: &[u8]) -> Result<AccountSet> {
    parse_accounts_response_from("", status, body)
}

/// Parses a claim response body into access credentials. Non-200 statuses
/// and empty bodies become [`SimplefinError::Api`].
pub fn parse_claim_response(status: u16, body: &[u8]) -> Result<AccessCredentials> {
    parse_claim_response_from("", status, body)
}

/// [`parse_accounts_response`] that records `uri` in API errors.
pub(crate) fn parse_accounts_response_from(
    uri: &str,
    status: u16,
    body: &[u8],
) -> Result<AccountSet> {
    if status != 200 {
        return Err(api_error(uri, status, "failed to fetch accounts", body));
    }
    serde_json::from_slice(body).map_err(|e| SimplefinError::DataFormat {
        message: "failed to parse accounts response".into(),
        source: Some(Box::new(e)),
    })
}

/// [`parse_claim_response`] that records `uri` in API errors.
pub(crate) fn parse_claim_response_from(
    uri: &str,
    status: u16,
    body: &[u8],
) -> Result<AccessCredentials> {
    if status != 200 {
        return Err(api_error(uri, status, "failed to claim access URL", body));
    }
    let access_url = String::from_utf8_lossy(body).trim().to_string();
    if access_url.is_empty() {
        return Err(api_error(
            uri,
            status,
            "claim response did not include an access URL",
            b"",
        ));
    }
    AccessCredentials::parse(&access_url)
}

/// An [`SimplefinError::Api`] carrying the (lossily decoded) response body.
pub(crate) fn api_error(uri: &str, status: u16, message: &str, body: &[u8]) -> SimplefinError {
    SimplefinError::Api {
        uri: uri.to_string(),
        status_code: status,
        message: message.to_string(),
        response_body: String::from_utf8_lossy(body).to_string(),
    }
}
