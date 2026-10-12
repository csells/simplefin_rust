//! Talking to SimpleFIN: claiming a setup token and collecting accounts.
//!
//! The collection pipeline is the library's; only the HTTP transport differs.
//! Natively it is the library's Asupersync client on a background thread
//! ([`crate::background`]); in a browser it is GPUI's `fetch`-based client.

use simplefin::{AccountFetcher, collect_data};

use crate::files::SharedFiles;

/// Collects new data into `files` using `fetcher`; returns a one-line summary.
pub async fn collect(fetcher: &impl AccountFetcher, files: SharedFiles, now: i64) -> anyhow::Result<String> {
    let mut storage = files.storage()?;
    let report = collect_data(fetcher, &mut storage, now, &mut |event| {
        log::info!("sync: {event:?}");
    })
    .await?;
    Ok(summary(report.new_transactions, report.accounts))
}

fn looks_like_url(input: &str) -> bool {
    input.starts_with("https://") || input.starts_with("http://")
}

fn summary(new_transactions: usize, accounts: usize) -> String {
    let txns = if new_transactions == 1 { "transaction" } else { "transactions" };
    let accts = if accounts == 1 { "account" } else { "accounts" };
    format!("{new_transactions} new {txns} across {accounts} {accts}")
}

/// Epoch seconds (works in browsers, where `std::time::SystemTime` panics).
pub fn now() -> i64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use asupersync::Cx;
    use simplefin::{AccessClient, AccessCredentials, BridgeClient};

    use crate::files::SharedFiles;

    /// An access URL is validated as-is; anything else is claimed (once) as a
    /// setup token at the bridge it names.
    pub async fn resolve_access_url(cx: &Cx, token_or_url: &str) -> anyhow::Result<String> {
        let input = token_or_url.trim();
        if super::looks_like_url(input) {
            return Ok(AccessCredentials::parse(input)?.access_url);
        }
        let bridge = BridgeClient::new(None, None);
        Ok(bridge.claim_access_credentials(cx, input).await?.access_url)
    }

    pub async fn sync(cx: &Cx, access_url: &str, files: SharedFiles) -> anyhow::Result<String> {
        let client = AccessClient::new(AccessCredentials::parse(access_url)?, None);
        super::collect(&client.fetcher(cx), files, super::now()).await
    }
}

#[cfg(target_arch = "wasm32")]
pub mod web {
    use std::sync::Arc;

    use futures::AsyncReadExt as _;
    use gpui::http_client::{AsyncBody, HttpClient, Request};
    use simplefin::{
        AccessCredentials, AccountFetcher, AccountQueryParams, AccountSet, HttpRequestSpec,
        SetupToken, parse_accounts_response, parse_claim_response,
    };

    use crate::files::SharedFiles;

    /// Sends a request spec through GPUI's HTTP client; returns status and body.
    async fn send(http: &Arc<dyn HttpClient>, spec: HttpRequestSpec) -> anyhow::Result<(u16, Vec<u8>)> {
        let mut request = Request::builder().method(spec.method).uri(spec.url.as_str());
        for (name, value) in &spec.headers {
            request = request.header(name.as_str(), value.as_str());
        }
        let response = http.send(request.body(AsyncBody::empty())?).await?;
        let status = response.status().as_u16();
        let mut body = Vec::new();
        response.into_body().read_to_end(&mut body).await?;
        Ok((status, body))
    }

    /// SimpleFIN accounts over the browser's `fetch` (the bridge allows CORS).
    pub struct FetchAccounts {
        http: Arc<dyn HttpClient>,
        credentials: AccessCredentials,
    }

    impl AccountFetcher for FetchAccounts {
        async fn fetch_accounts(&self, params: &AccountQueryParams) -> simplefin::Result<AccountSet> {
            let spec = self.credentials.accounts_request(params)?;
            let (status, body) = send(&self.http, spec)
                .await
                .map_err(|e| simplefin::SimplefinError::Transport(format!("{e:#}")))?;
            parse_accounts_response(status, &body)
        }
    }

    pub async fn resolve_access_url(http: Arc<dyn HttpClient>, token_or_url: &str) -> anyhow::Result<String> {
        let input = token_or_url.trim();
        if super::looks_like_url(input) {
            return Ok(AccessCredentials::parse(input)?.access_url);
        }
        let spec = SetupToken::parse(input)?.claim_request();
        let (status, body) = send(&http, spec).await?;
        Ok(parse_claim_response(status, &body)?.access_url)
    }

    pub async fn sync(http: Arc<dyn HttpClient>, access_url: &str, files: SharedFiles) -> anyhow::Result<String> {
        let fetcher = FetchAccounts { http, credentials: AccessCredentials::parse(access_url)? };
        super::collect(&fetcher, files, super::now()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_are_not_setup_tokens() {
        assert!(looks_like_url("https://user:pass@bridge.example/simplefin"));
        assert!(!looks_like_url("aHR0cHM6Ly9icmlkZ2UuZXhhbXBsZS9jbGFpbS9YWVo="));
    }

    #[test]
    fn summary_pluralizes() {
        assert_eq!(summary(1, 1), "1 new transaction across 1 account");
        assert_eq!(summary(0, 3), "0 new transactions across 3 accounts");
    }
}
