//! Transport-independent request building and response parsing.

use base64::{Engine as _, engine::general_purpose};
use simplefin::{
    AccessCredentials, AccountQueryParams, HttpRequestSpec, SetupToken, SimplefinError,
    parse_accounts_response, parse_claim_response,
};

const ACCESS_URL: &str = "https://user:secret@bridge.example.com/simplefin";

fn creds() -> AccessCredentials {
    AccessCredentials::parse(ACCESS_URL).unwrap()
}

fn header<'a>(spec: &'a HttpRequestSpec, name: &str) -> Option<&'a str> {
    spec.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

// ── accounts_request ────────────────────────────────────────────────────────

#[test]
fn accounts_request_is_get_without_credentials_in_url() {
    let spec = creds()
        .accounts_request(&AccountQueryParams::default())
        .unwrap();
    assert_eq!(spec.method, "GET");
    assert_eq!(spec.url, "https://bridge.example.com/simplefin/accounts");
    assert!(!spec.url.contains("user"), "username must not be in URL");
    assert!(!spec.url.contains("secret"), "password must not be in URL");
    assert!(!spec.url.contains('@'));
}

#[test]
fn accounts_request_carries_basic_auth_and_accept() {
    let creds = creds();
    let spec = creds
        .accounts_request(&AccountQueryParams::default())
        .unwrap();
    let expected = format!("Basic {}", general_purpose::STANDARD.encode("user:secret"));
    assert_eq!(header(&spec, "Authorization"), Some(expected.as_str()));
    assert_eq!(
        header(&spec, "Authorization"),
        Some(creds.basic_auth_header_value().as_str())
    );
    assert_eq!(header(&spec, "Accept"), Some("application/json"));
}

#[test]
fn accounts_request_builds_query() {
    let spec = creds()
        .accounts_request(&AccountQueryParams {
            start_date: Some(100),
            end_date: Some(200),
            include_pending: true,
            account_ids: Some(vec!["a1".into(), String::new(), "a 2".into()]),
            balances_only: true,
        })
        .unwrap();
    assert_eq!(
        spec.url,
        "https://bridge.example.com/simplefin/accounts\
         ?start-date=100&end-date=200&pending=1&balances-only=1&account=a1&account=a+2"
    );
}

#[test]
fn accounts_request_omits_unset_query() {
    let spec = creds()
        .accounts_request(&AccountQueryParams {
            balances_only: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        spec.url,
        "https://bridge.example.com/simplefin/accounts?balances-only=1"
    );
}

#[test]
fn accounts_request_allows_equal_dates() {
    let spec = creds().accounts_request(&AccountQueryParams {
        start_date: Some(100),
        end_date: Some(100),
        ..Default::default()
    });
    assert!(spec.is_ok());
}

#[test]
fn accounts_request_rejects_inverted_dates() {
    let err = creds()
        .accounts_request(&AccountQueryParams {
            start_date: Some(200),
            end_date: Some(100),
            ..Default::default()
        })
        .unwrap_err();
    assert!(matches!(err, SimplefinError::InvalidArgument(_)));
    assert!(err.to_string().contains("start_date (200)"));
}

// ── parse_accounts_response ────────────────────────────────────────────────

#[test]
fn parse_accounts_response_ok() {
    let body = br#"{"errors":["bridge note"],"accounts":[]}"#;
    let set = parse_accounts_response(200, body).unwrap();
    assert!(set.accounts.is_empty());
    assert_eq!(set.server_messages, vec!["bridge note"]);
}

#[test]
fn parse_accounts_response_maps_non_200_to_api_error() {
    let err = parse_accounts_response(403, b"forbidden").unwrap_err();
    match err {
        SimplefinError::Api {
            status_code,
            message,
            response_body,
            ..
        } => {
            assert_eq!(status_code, 403);
            assert_eq!(message, "failed to fetch accounts");
            assert_eq!(response_body, "forbidden");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[test]
fn parse_accounts_response_maps_bad_json_to_data_format() {
    let err = parse_accounts_response(200, b"not json").unwrap_err();
    match err {
        SimplefinError::DataFormat { message, .. } => {
            assert_eq!(message, "failed to parse accounts response");
        }
        other => panic!("expected DataFormat error, got {other:?}"),
    }
}

// ── claim_request ───────────────────────────────────────────────────────────

#[test]
fn claim_request_posts_to_claim_url() {
    let claim_url = "https://bridge.example.com/simplefin/claim/token-123";
    let token = SetupToken::parse(&general_purpose::STANDARD.encode(claim_url)).unwrap();
    let spec = token.claim_request();
    assert_eq!(spec.method, "POST");
    assert_eq!(spec.url, claim_url);
    assert_eq!(
        spec.headers,
        vec![("Accept".to_string(), "text/plain".to_string())]
    );
}

// ── parse_claim_response ───────────────────────────────────────────────────

#[test]
fn parse_claim_response_ok_trims_body() {
    let creds = parse_claim_response(200, format!("  {ACCESS_URL}\n").as_bytes()).unwrap();
    assert_eq!(creds.access_url, ACCESS_URL);
    assert_eq!(creds.username, "user");
    assert_eq!(creds.password, "secret");
}

#[test]
fn parse_claim_response_maps_non_200_to_api_error() {
    let err = parse_claim_response(403, b"already claimed").unwrap_err();
    match err {
        SimplefinError::Api {
            status_code,
            message,
            response_body,
            ..
        } => {
            assert_eq!(status_code, 403);
            assert_eq!(message, "failed to claim access URL");
            assert_eq!(response_body, "already claimed");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[test]
fn parse_claim_response_rejects_empty_body() {
    let err = parse_claim_response(200, b"  \n").unwrap_err();
    match err {
        SimplefinError::Api {
            status_code,
            message,
            ..
        } => {
            assert_eq!(status_code, 200);
            assert_eq!(message, "claim response did not include an access URL");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[test]
fn parse_claim_response_rejects_invalid_access_url() {
    let err = parse_claim_response(200, b"https://no-credentials.example.com").unwrap_err();
    assert!(matches!(err, SimplefinError::DataFormat { .. }));
}
