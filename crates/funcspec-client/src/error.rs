use std::time::Duration;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    /// 401 — invalid or expired token
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// 403 — permission denied
    #[error("Permission denied: {0}")]
    Forbidden(String),

    /// 404 — resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// 422 — validation failure
    #[error("Validation error: {0}")]
    Validation(String),

    /// 429 — rate limited
    #[error("Rate limited — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    /// 5xx — server-side error
    #[error("Server error ({status}): {message}")]
    Server { status: u16, message: String },

    /// Network-level failure (DNS, connection refused, etc.)
    #[error("Network error: {0}")]
    Network(String),

    /// Request timed out
    #[error("Request timed out after {secs}s")]
    Timeout { secs: u64 },

    /// JSON parse failure
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Any other error
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Returns true if the operation can be safely retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::RateLimited { .. } => true,
            Error::Server { status, .. } => *status >= 500,
            Error::Network(_) => true,
            Error::Timeout { .. } => true,
            _ => false,
        }
    }

    /// Returns the recommended retry delay, if any.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::RateLimited { retry_after_secs } => {
                Some(Duration::from_secs(*retry_after_secs))
            }
            _ => None,
        }
    }

    /// Map an HTTP response into a typed error.
    pub async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let status_u16 = status.as_u16();

        // Parse Retry-After header before consuming body
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(10);

        let body = response.text().await.unwrap_or_default();

        let message = extract_error_message(&body);

        match status_u16 {
            401 => Error::Auth(if message.is_empty() {
                "Not authenticated. Run `funcspec auth login` to connect.".into()
            } else {
                message
            }),
            403 => Error::Forbidden(if message.is_empty() {
                "Permission denied. You don't have access to this resource.".into()
            } else {
                message
            }),
            404 => Error::NotFound(if message.is_empty() {
                "Resource not found.".into()
            } else {
                message
            }),
            422 => Error::Validation(if message.is_empty() {
                body
            } else {
                message
            }),
            429 => Error::RateLimited {
                retry_after_secs: retry_after,
            },
            s if s >= 500 => Error::Server {
                status: status_u16,
                message: if message.is_empty() {
                    format!("Server returned {s}")
                } else {
                    message
                },
            },
            _ => Error::Other(format!("Unexpected status {status_u16}: {message}")),
        }
    }

    /// User-friendly CLI message.
    pub fn user_message(&self) -> String {
        match self {
            Error::Auth(_) => {
                "Not authenticated. Run `funcspec auth login` to connect.".into()
            }
            Error::Forbidden(msg) => format!("Permission denied. {msg}"),
            Error::NotFound(msg) => msg.clone(),
            Error::Validation(msg) => format!("Validation error: {msg}"),
            Error::RateLimited { retry_after_secs } => {
                format!("Rate limited. Retry in {retry_after_secs}s.")
            }
            Error::Server { status, message } => {
                format!("Server error ({status}): {message}")
            }
            Error::Network(_) => {
                "Cannot reach funcspec.net. Check your connection.".into()
            }
            Error::Timeout { secs } => {
                format!("Request timed out after {secs}s. Check your connection or increase --timeout.")
            }
            Error::Json(e) => format!("Failed to parse server response: {e}"),
            Error::Other(msg) => msg.clone(),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Error::Timeout { secs: 30 }
        } else {
            Error::Network(e.to_string())
        }
    }
}

fn extract_error_message(body: &str) -> String {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
        // Try common error fields: "error", "message", "errors"
        if let Some(msg) = val.get("error").and_then(|v| v.as_str()) {
            return msg.to_string();
        }
        if let Some(msg) = val.get("message").and_then(|v| v.as_str()) {
            return msg.to_string();
        }
        if let Some(errors) = val.get("errors") {
            return errors.to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_is_not_retryable() {
        let e = Error::Auth("bad token".into());
        assert!(!e.is_retryable());
    }

    #[test]
    fn rate_limited_is_retryable() {
        let e = Error::RateLimited { retry_after_secs: 5 };
        assert!(e.is_retryable());
    }

    #[test]
    fn rate_limited_retry_after() {
        let e = Error::RateLimited { retry_after_secs: 42 };
        assert_eq!(e.retry_after(), Some(Duration::from_secs(42)));
    }

    #[test]
    fn non_rate_limited_no_retry_after() {
        let e = Error::NotFound("x".into());
        assert_eq!(e.retry_after(), None);
    }

    #[test]
    fn network_error_is_retryable() {
        let e = Error::Network("dns failed".into());
        assert!(e.is_retryable());
    }

    #[test]
    fn timeout_is_retryable() {
        let e = Error::Timeout { secs: 30 };
        assert!(e.is_retryable());
    }

    #[test]
    fn server_error_is_retryable() {
        let e = Error::Server { status: 500, message: "oops".into() };
        assert!(e.is_retryable());
    }

    #[test]
    fn validation_not_retryable() {
        let e = Error::Validation("bad field".into());
        assert!(!e.is_retryable());
    }

    #[test]
    fn user_message_auth() {
        let e = Error::Auth("x".into());
        assert!(e.user_message().contains("funcspec auth login"));
    }

    #[test]
    fn user_message_network() {
        let e = Error::Network("timeout".into());
        assert!(e.user_message().contains("funcspec.net"));
    }

    #[test]
    fn extract_error_from_json_error_field() {
        let body = r#"{"error": "invalid token"}"#;
        assert_eq!(extract_error_message(body), "invalid token");
    }

    #[test]
    fn extract_error_from_json_message_field() {
        let body = r#"{"message": "not found"}"#;
        assert_eq!(extract_error_message(body), "not found");
    }

    #[test]
    fn extract_error_from_non_json() {
        let body = "plain text error";
        assert_eq!(extract_error_message(body), "");
    }
}
