use thiserror::Error;

#[derive(Debug, Error)]
pub enum SimplefinError {
    #[error("invalid setup token: {message}")]
    InvalidSetupToken {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("data format error: {message}")]
    DataFormat {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("API error (status {status_code}): {message}")]
    Api {
        uri: String,
        status_code: u16,
        message: String,
        response_body: String,
    },

    #[error("HTTP error: {0}")]
    Http(#[from] asupersync::http::h1::ClientError),

    #[error("{0}")]
    InvalidArgument(String),

    #[error("storage error: {message}")]
    Storage {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

pub type Result<T> = std::result::Result<T, SimplefinError>;
