use aws_credential_types::Credentials;
use reqwest::Url;
use std::{fmt, net::IpAddr};

/// Static credentials are never loaded from instance metadata or exposed by Debug.
pub struct S3Configuration {
    pub(super) endpoint: Url,
    pub(super) bucket: String,
    pub(super) prefix: String,
    pub(super) region: String,
    pub(super) credentials: Credentials,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfigurationError(pub(super) &'static str);
impl fmt::Display for ConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}
impl std::error::Error for ConfigurationError {}

pub struct S3Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

impl S3Configuration {
    pub fn new(
        endpoint: &str,
        bucket: &str,
        prefix: &str,
        region: &str,
        credentials: S3Credentials,
        allow_http_loopback: bool,
    ) -> Result<Self, ConfigurationError> {
        let endpoint = validate_endpoint(endpoint, allow_http_loopback)?;
        validate_bucket(bucket)?;
        if prefix.len() > 128 || prefix.split('/').any(|part| !identifier(part, 64)) {
            return Err(ConfigurationError(
                "S3_PREFIX must contain bounded, nonempty ASCII path segments",
            ));
        }
        if !identifier(region, 64) {
            return Err(ConfigurationError(
                "S3_REGION must be a nonempty ASCII region identifier",
            ));
        }
        if !credential(&credentials.access_key_id, 128)
            || !credential(&credentials.secret_access_key, 256)
            || credentials
                .session_token
                .as_ref()
                .is_some_and(|value| !credential(value, 4096))
        {
            return Err(ConfigurationError(
                "S3 credentials must be nonempty bounded ASCII values",
            ));
        }
        Ok(Self {
            endpoint,
            bucket: bucket.into(),
            prefix: prefix.into(),
            region: region.into(),
            credentials: Credentials::new(
                credentials.access_key_id,
                credentials.secret_access_key,
                credentials.session_token,
                None,
                "rustsurvey-explicit",
            ),
        })
    }
}

fn identifier(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn credential(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn validate_endpoint(value: &str, allow_http_loopback: bool) -> Result<Url, ConfigurationError> {
    let error = || {
        ConfigurationError(
            "S3_ENDPOINT must be an HTTPS origin without user info, path, query or fragment",
        )
    };
    let url = Url::parse(value).map_err(|_| error())?;
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(error());
    }
    if url.scheme() == "https" {
        return Ok(url);
    }
    let loopback = url
        .host_str()
        .and_then(|host| host.trim_matches(['[', ']']).parse::<IpAddr>().ok())
        .is_some_and(|address| address.is_loopback());
    if url.scheme() == "http" && allow_http_loopback && loopback {
        return Ok(url);
    }
    Err(error())
}

fn validate_bucket(value: &str) -> Result<(), ConfigurationError> {
    let valid = (3..=63).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
        })
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && !value.contains("..")
        && !value.contains(".-")
        && !value.contains("-.")
        && value.parse::<IpAddr>().is_err();
    if valid {
        return Ok(());
    }
    Err(ConfigurationError(
        "S3_BUCKET must be a DNS-compatible bucket name",
    ))
}
