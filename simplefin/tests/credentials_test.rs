use base64::{Engine as _, engine::general_purpose};

use simplefin::{AccessCredentials, SetupToken};

// ── SetupToken ──────────────────────────────────────────────────────────────

#[test]
fn setup_token_valid_https_url() {
    let claim_url = "https://bridge.simplefin.org/simplefin/claim/abc123";
    let token = general_purpose::STANDARD.encode(claim_url);
    let parsed = SetupToken::parse(&token).unwrap();
    assert_eq!(parsed.value, token);
    assert_eq!(parsed.claim_url.as_str(), claim_url);
}

#[test]
fn setup_token_valid_http_url() {
    let claim_url = "http://localhost:8080/claim/test-token";
    let token = general_purpose::STANDARD.encode(claim_url);
    let parsed = SetupToken::parse(&token).unwrap();
    assert_eq!(parsed.claim_url.as_str(), claim_url);
}

#[test]
fn setup_token_url_safe_base64() {
    // URL-safe Base64 uses - and _ instead of + and /
    let claim_url = "https://bridge.simplefin.org/simplefin/claim/token+with+special/chars";
    let token = general_purpose::URL_SAFE.encode(claim_url);
    let parsed = SetupToken::parse(&token).unwrap();
    assert_eq!(parsed.claim_url.as_str(), claim_url);
}

#[test]
fn setup_token_trims_whitespace() {
    let claim_url = "https://bridge.simplefin.org/simplefin/claim/trimmed";
    let token = general_purpose::STANDARD.encode(claim_url);
    let padded = format!("  {token}  \n");
    let parsed = SetupToken::parse(&padded).unwrap();
    assert_eq!(parsed.claim_url.as_str(), claim_url);
}

#[test]
fn setup_token_rejects_empty() {
    let result = SetupToken::parse("");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("must not be empty"));
}

#[test]
fn setup_token_rejects_whitespace_only() {
    let result = SetupToken::parse("   \n\t  ");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("must not be empty"));
}

#[test]
fn setup_token_rejects_invalid_base64() {
    let result = SetupToken::parse("!!!not-valid-base64!!!");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Base64"));
}

#[test]
fn setup_token_rejects_non_url_content() {
    let token = general_purpose::STANDARD.encode("just plain text, not a URL");
    let result = SetupToken::parse(&token);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not a valid URL"));
}

#[test]
fn setup_token_rejects_ftp_scheme() {
    let token = general_purpose::STANDARD.encode("ftp://example.com/claim/token");
    let result = SetupToken::parse(&token);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("http or https"));
}

#[test]
fn setup_token_rejects_file_scheme() {
    let token = general_purpose::STANDARD.encode("file:///etc/passwd");
    let result = SetupToken::parse(&token);
    assert!(result.is_err());
}

#[test]
fn setup_token_with_port() {
    let claim_url = "https://bridge.simplefin.org:8443/simplefin/claim/token";
    let token = general_purpose::STANDARD.encode(claim_url);
    let parsed = SetupToken::parse(&token).unwrap();
    assert_eq!(parsed.claim_url.port(), Some(8443));
}

#[test]
fn setup_token_with_path_segments() {
    let claim_url = "https://bridge.simplefin.org/simplefin/claim/abc/def/ghi";
    let token = general_purpose::STANDARD.encode(claim_url);
    let parsed = SetupToken::parse(&token).unwrap();
    assert_eq!(parsed.claim_url.path(), "/simplefin/claim/abc/def/ghi");
}

// ── AccessCredentials ───────────────────────────────────────────────────────

#[test]
fn access_credentials_standard_url() {
    let url = "https://user123:pass456@api.simplefin.org/simplefin";
    let creds = AccessCredentials::parse(url).unwrap();
    assert_eq!(creds.access_url, url);
    assert_eq!(creds.username, "user123");
    assert_eq!(creds.password, "pass456");
    assert_eq!(creds.base_url.scheme(), "https");
    assert_eq!(creds.base_url.host_str(), Some("api.simplefin.org"));
    assert_eq!(creds.base_url.path(), "/simplefin");
    // base_url should NOT contain credentials
    assert_eq!(creds.base_url.username(), "");
    assert!(creds.base_url.password().is_none());
}

#[test]
fn access_credentials_percent_encoded_username() {
    let url = "https://user%40domain:pass@api.simplefin.org/simplefin";
    let creds = AccessCredentials::parse(url).unwrap();
    assert_eq!(creds.username, "user@domain");
    assert_eq!(creds.password, "pass");
}

#[test]
fn access_credentials_percent_encoded_password() {
    let url = "https://user:p%40ss%3Aw0rd@api.simplefin.org/simplefin";
    let creds = AccessCredentials::parse(url).unwrap();
    assert_eq!(creds.username, "user");
    assert_eq!(creds.password, "p@ss:w0rd");
}

#[test]
fn access_credentials_empty_password() {
    let url = "https://user@api.simplefin.org/simplefin";
    let creds = AccessCredentials::parse(url).unwrap();
    assert_eq!(creds.username, "user");
    assert_eq!(creds.password, "");
}

#[test]
fn access_credentials_with_port() {
    let url = "https://user:pass@api.simplefin.org:8443/simplefin";
    let creds = AccessCredentials::parse(url).unwrap();
    assert_eq!(creds.base_url.port(), Some(8443));
    assert_eq!(creds.username, "user");
}

#[test]
fn access_credentials_trims_whitespace() {
    let url = "  https://user:pass@api.simplefin.org/simplefin  \n";
    let creds = AccessCredentials::parse(url).unwrap();
    assert_eq!(creds.username, "user");
    assert_eq!(creds.access_url, url.trim());
}

#[test]
fn access_credentials_rejects_empty() {
    let result = AccessCredentials::parse("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must not be empty"));
}

#[test]
fn access_credentials_rejects_whitespace_only() {
    let result = AccessCredentials::parse("   ");
    assert!(result.is_err());
}

#[test]
fn access_credentials_rejects_no_credentials() {
    let result = AccessCredentials::parse("https://api.simplefin.org/simplefin");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Basic Auth"));
}

#[test]
fn access_credentials_rejects_invalid_url() {
    let result = AccessCredentials::parse("not a url at all");
    assert!(result.is_err());
}

// ── basic_auth_header_value ─────────────────────────────────────────────────

#[test]
fn basic_auth_header_value_format() {
    let url = "https://myuser:mypass@api.simplefin.org/simplefin";
    let creds = AccessCredentials::parse(url).unwrap();
    let header = creds.basic_auth_header_value();
    assert!(header.starts_with("Basic "));

    // Decode and verify
    let encoded = header.strip_prefix("Basic ").unwrap();
    let decoded = general_purpose::STANDARD.decode(encoded).unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();
    assert_eq!(decoded_str, "myuser:mypass");
}

#[test]
fn basic_auth_header_with_special_chars() {
    let url = "https://user%40example:p%40ss%3Fw0rd@api.simplefin.org/simplefin";
    let creds = AccessCredentials::parse(url).unwrap();
    let header = creds.basic_auth_header_value();
    let encoded = header.strip_prefix("Basic ").unwrap();
    let decoded = general_purpose::STANDARD.decode(encoded).unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();
    assert_eq!(decoded_str, "user@example:p@ss?w0rd");
}

// ── endpoint_url ────────────────────────────────────────────────────────────

#[test]
fn endpoint_url_single_segment() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin",
    ).unwrap();
    let url = creds.endpoint_url(&["accounts"], None);
    assert_eq!(url.path(), "/simplefin/accounts");
    assert_eq!(url.host_str(), Some("api.simplefin.org"));
    // No credentials in the URL
    assert_eq!(url.username(), "");
}

#[test]
fn endpoint_url_multiple_segments() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin",
    ).unwrap();
    let url = creds.endpoint_url(&["accounts", "details"], None);
    assert_eq!(url.path(), "/simplefin/accounts/details");
}

#[test]
fn endpoint_url_with_query_params() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin",
    ).unwrap();
    let url = creds.endpoint_url(
        &["accounts"],
        Some(&[("start-date", "1706745600"), ("pending", "1")]),
    );
    assert_eq!(url.path(), "/simplefin/accounts");
    let query = url.query().unwrap();
    assert!(query.contains("start-date=1706745600"));
    assert!(query.contains("pending=1"));
}

#[test]
fn endpoint_url_with_empty_query_params() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin",
    ).unwrap();
    let url = creds.endpoint_url(&["accounts"], Some(&[]));
    assert!(url.query().is_none());
}

#[test]
fn endpoint_url_no_query_params() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin",
    ).unwrap();
    let url = creds.endpoint_url(&["accounts"], None);
    assert!(url.query().is_none());
}

#[test]
fn endpoint_url_preserves_trailing_slash_base() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin/",
    ).unwrap();
    let url = creds.endpoint_url(&["accounts"], None);
    assert!(url.path().contains("accounts"));
}

#[test]
fn endpoint_url_with_multiple_same_key_params() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin",
    ).unwrap();
    let url = creds.endpoint_url(
        &["accounts"],
        Some(&[("account", "acct-1"), ("account", "acct-2"), ("account", "acct-3")]),
    );
    let query = url.query().unwrap();
    assert!(query.contains("account=acct-1"));
    assert!(query.contains("account=acct-2"));
    assert!(query.contains("account=acct-3"));
}

// ── Clone ───────────────────────────────────────────────────────────────────

#[test]
fn access_credentials_clone() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin",
    ).unwrap();
    let cloned = creds.clone();
    assert_eq!(creds.access_url, cloned.access_url);
    assert_eq!(creds.username, cloned.username);
    assert_eq!(creds.password, cloned.password);
    assert_eq!(creds.base_url, cloned.base_url);
}
