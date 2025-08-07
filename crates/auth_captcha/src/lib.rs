use config_manager::get_config;
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptchaError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("invalid captcha response")]
    Invalid,
}

#[derive(Deserialize)]
struct RecaptchaResponse {
    success: bool,
    #[serde(default)]
    challenge_ts: String,
    #[serde(default)]
    hostname: String,
}

pub struct CaptchaVerifier {
    client: Client,
    url: String,
    secret: String,
}

impl CaptchaVerifier {
    pub fn new() -> Self {
        let cfg = get_config().captcha.clone();
        Self {
            client: Client::new(),
            url: cfg.verify_url,
            secret: cfg.secret_key,
        }
    }

    pub async fn verify(&self, token: &str) -> Result<(), CaptchaError> {
        let resp: RecaptchaResponse = self
            .client
            .post(&self.url)
            .form(&[("secret", &self.secret), ("response", &token.to_string())])
            .send()
            .await?
            .json()
            .await?;
        if resp.success {
            Ok(())
        } else {
            Err(CaptchaError::Invalid)
        }
    }
}

#[cfg(test)]
impl CaptchaVerifier {
    pub fn test_new(url: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            url: url.into(),
            secret: secret.into(),
        }
    }
}
