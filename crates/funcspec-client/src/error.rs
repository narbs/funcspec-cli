use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Server error ({status}): {message}")]
    Server { status: u16, message: String },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Map an HTTP response status to a typed error.
    pub async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();

        // Try to extract message from JSON error response
        let message = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
            .unwrap_or(body);

        match status {
            401 => Error::Auth(message),
            403 => Error::Auth(format!("Permission denied: {message}")),
            404 => Error::NotFound(message),
            422 => Error::Validation(message),
            429 => {
                // TODO: parse Retry-After header
                Error::RateLimited {
                    retry_after_secs: 10,
                }
            }
            _ => Error::Server { status, message },
        }
    }
}
