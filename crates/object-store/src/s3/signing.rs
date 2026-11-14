use super::S3Configuration;
use crate::StoreError;
use aws_sigv4::{
    http_request::{
        PayloadChecksumKind, PercentEncodingMode, SignableBody, SignableRequest, SigningSettings,
        UriPathNormalizationMode, sign,
    },
    sign::v4,
};
use reqwest::{Request, header::HeaderValue};
use std::time::SystemTime;

pub(super) fn apply(
    request: &mut Request,
    configuration: &S3Configuration,
    sha256: String,
    time: SystemTime,
) -> Result<(), StoreError> {
    let identity = configuration.credentials.clone().into();
    let mut settings = SigningSettings::default();
    settings.percent_encoding_mode = PercentEncodingMode::Single;
    settings.uri_path_normalization_mode = UriPathNormalizationMode::Disabled;
    settings.payload_checksum_kind = PayloadChecksumKind::XAmzSha256;
    let parameters = v4::SigningParams::builder()
        .identity(&identity)
        .region(&configuration.region)
        .name("s3")
        .time(time)
        .settings(settings)
        .build()
        .map_err(|_| StoreError::Unavailable)?
        .into();
    let headers = request
        .headers()
        .iter()
        .map(|(name, value)| {
            value
                .to_str()
                .map(|value| (name.as_str(), value))
                .map_err(|_| StoreError::Unavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let signable = SignableRequest::new(
        request.method().as_str(),
        request.url().as_str(),
        headers.into_iter(),
        SignableBody::Precomputed(sha256),
    )
    .map_err(|_| StoreError::Unavailable)?;
    let (instructions, _) = sign(signable, &parameters)
        .map_err(|_| StoreError::Unavailable)?
        .into_parts();
    for (name, value) in instructions.headers() {
        let mut header = HeaderValue::from_str(value).map_err(|_| StoreError::Unavailable)?;
        header.set_sensitive(matches!(name, "authorization" | "x-amz-security-token"));
        request.headers_mut().insert(
            reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| StoreError::Unavailable)?,
            header,
        );
    }
    Ok(())
}
