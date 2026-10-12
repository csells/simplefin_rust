use std::future::Future;

use asupersync::Cx;
use asupersync::http::h1::HttpClient;

use crate::credentials::AccessCredentials;
use crate::error::Result;
use crate::models::AccountSet;
use crate::requests::{AccountQueryParams, parse_accounts_response_from};
use crate::sync::AccountFetcher;

/// Client that uses SimpleFIN access credentials to retrieve account data.
pub struct AccessClient {
    credentials: AccessCredentials,
    http: HttpClient,
}

impl AccessClient {
    /// Creates a client that issues requests with the provided credentials.
    pub fn new(credentials: AccessCredentials, user_agent: Option<&str>) -> Self {
        AccessClient {
            credentials,
            http: super::http_client(user_agent),
        }
    }

    /// Retrieves account and transaction data from the SimpleFIN server.
    pub async fn get_accounts(&self, cx: &Cx, params: &AccountQueryParams) -> Result<AccountSet> {
        let spec = self.credentials.accounts_request(params)?;
        let url = spec.url.clone();
        let response = super::send(&self.http, cx, spec).await?;
        parse_accounts_response_from(&url, response.status, &response.body)
    }

    /// An [`AccountFetcher`] that issues this client's requests under `cx`.
    pub fn fetcher<'a>(&'a self, cx: &'a Cx) -> AccessFetcher<'a> {
        AccessFetcher { client: self, cx }
    }
}

/// [`AccountFetcher`] adapter binding an [`AccessClient`] to a runtime context.
pub struct AccessFetcher<'a> {
    client: &'a AccessClient,
    cx: &'a Cx,
}

impl AccountFetcher for AccessFetcher<'_> {
    fn fetch_accounts(
        &self,
        params: &AccountQueryParams,
    ) -> impl Future<Output = Result<AccountSet>> {
        self.client.get_accounts(self.cx, params)
    }
}
