use base64::{Engine as _, engine::general_purpose};
use url::Url;

use crate::error::{SimplefinError, Result};

/// Representation of the temporary setup token that a user creates via the
/// SimpleFIN Bridge UI. The token is Base64 encoded and resolves to the
/// one-time claim URL when decoded.
#[derive(Debug)]
pub struct SetupToken {
    /// Original string supplied by the user.
    pub value: String,
    /// Claim endpoint resolved from the setup token.
    pub claim_url: Url,
}

impl SetupToken {
    /// Parses a Base64-encoded setup token string.
    pub fn parse(token: &str) -> Result<SetupToken> {
        let cleaned = token.trim();
        if cleaned.is_empty() {
            return Err(SimplefinError::InvalidSetupToken {
                message: "setup token must not be empty".into(),
                source: None,
            });
        }

        let decoded_bytes = general_purpose::STANDARD
            .decode(cleaned)
            .or_else(|_| general_purpose::URL_SAFE.decode(cleaned))
            .map_err(|e| SimplefinError::InvalidSetupToken {
                message: "failed to Base64-decode setup token".into(),
                source: Some(Box::new(e)),
            })?;

        let decoded = String::from_utf8(decoded_bytes).map_err(|e| {
            SimplefinError::InvalidSetupToken {
                message: "decoded setup token is not valid UTF-8".into(),
                source: Some(Box::new(e)),
            }
        })?;

        let claim_url = Url::parse(&decoded).map_err(|e| {
            SimplefinError::InvalidSetupToken {
                message: "decoded setup token is not a valid URL".into(),
                source: Some(Box::new(e)),
            }
        })?;

        if claim_url.scheme() != "https" && claim_url.scheme() != "http" {
            return Err(SimplefinError::InvalidSetupToken {
                message: format!(
                    "claim URL must use http or https scheme, got: {}",
                    claim_url.scheme()
                ),
                source: None,
            });
        }

        if claim_url.host().is_none() {
            return Err(SimplefinError::InvalidSetupToken {
                message: "claim URL must have a host".into(),
                source: None,
            });
        }

        Ok(SetupToken {
            value: cleaned.to_string(),
            claim_url,
        })
    }
}

/// Credentials extracted from a SimpleFIN Access URL. Access URLs embed the
/// HTTP basic auth username and password required to query account data.
#[derive(Debug, Clone)]
pub struct AccessCredentials {
    /// Raw access URL returned by the bridge claim endpoint.
    pub access_url: String,
    /// Base URL to which endpoint segments are appended.
    pub base_url: Url,
    /// Username embedded in the access URL for HTTP basic authentication.
    pub username: String,
    /// Password embedded in the access URL for HTTP basic authentication.
    pub password: String,
}

impl AccessCredentials {
    /// Parses a SimpleFIN access URL into credential components.
    pub fn parse(url: &str) -> Result<AccessCredentials> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(SimplefinError::DataFormat {
                message: "access URL must not be empty".into(),
                source: None,
            });
        }

        let parsed = Url::parse(trimmed).map_err(|e| SimplefinError::DataFormat {
            message: "access URL is not a valid URL".into(),
            source: Some(Box::new(e)),
        })?;

        let raw_username = parsed.username();
        let raw_password = parsed.password().unwrap_or("");

        if raw_username.is_empty() {
            return Err(SimplefinError::DataFormat {
                message: "access URL must contain Basic Auth credentials".into(),
                source: None,
            });
        }

        let username =
            percent_encoding::percent_decode_str(raw_username)
                .decode_utf8()
                .map_err(|e| SimplefinError::DataFormat {
                    message: "username is not valid UTF-8".into(),
                    source: Some(Box::new(e)),
                })?
                .into_owned();

        let password =
            percent_encoding::percent_decode_str(raw_password)
                .decode_utf8()
                .map_err(|e| SimplefinError::DataFormat {
                    message: "password is not valid UTF-8".into(),
                    source: Some(Box::new(e)),
                })?
                .into_owned();

        // Build base URL without credentials
        let mut base_url = parsed.clone();
        base_url.set_username("").map_err(|()| SimplefinError::DataFormat {
            message: "failed to clear username from access URL".into(),
            source: None,
        })?;
        base_url.set_password(None).map_err(|()| SimplefinError::DataFormat {
            message: "failed to clear password from access URL".into(),
            source: None,
        })?;

        Ok(AccessCredentials {
            access_url: trimmed.to_string(),
            base_url,
            username,
            password,
        })
    }

    /// Returns the value for the `Authorization` header.
    pub fn basic_auth_header_value(&self) -> String {
        let encoded = general_purpose::STANDARD.encode(format!("{}:{}", self.username, self.password));
        format!("Basic {encoded}")
    }

    /// Builds a URL for an endpoint relative to the access URL.
    pub fn endpoint_url(
        &self,
        segments: &[&str],
        query_params: Option<&[(&str, &str)]>,
    ) -> Url {
        let mut url = self.base_url.clone();

        // Append path segments
        {
            let mut path = url.path().to_string();
            for segment in segments {
                if !path.ends_with('/') {
                    path.push('/');
                }
                path.push_str(segment);
            }
            url.set_path(&path);
        }

        // Add query parameters
        if let Some(params) = query_params
            && !params.is_empty()
        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in params {
                pairs.append_pair(key, value);
            }
        }

        url
    }
}
